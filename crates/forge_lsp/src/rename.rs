//! `RenameProvider` — `textDocument/rename`.
//!
//! The LSP rename request returns a `WorkspaceEdit` (a map of
//! `uri -> TextEdit[]`). This provider forwards the request and
//! returns the parsed edit. Preview vs. apply is a caller concern —
//! the provider is intentionally stateless (no workspace file writes
//! here).

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::lsp_client::{LspClient, LspRequest, Position, Range, TextDocumentIdentifier};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WorkspaceEdit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub changes: Option<HashMap<String, Vec<TextEdit>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameError { ServerError(String), InvalidResponse(String) }
impl std::fmt::Display for RenameError { fn fmt(&self, f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{ match self{Self::ServerError(m)=>write!(f,"lsp server error: {m}"), Self::InvalidResponse(m)=>write!(f,"invalid rename response: {m}")} } }
impl std::error::Error for RenameError {}
pub type RenameResult = Result<Option<WorkspaceEdit>, RenameError>;

#[derive(Debug, Clone, Serialize)]
struct RenameParams { #[serde(rename="textDocument")] text_document: TextDocumentIdentifier, position: Position, #[serde(rename="newName")] new_name: String }

pub struct RenameProvider { client: Box<dyn LspClient> }
impl RenameProvider {
    pub fn new(client: Box<dyn LspClient>) -> Self { Self{client} }
    pub fn name(&self)->&'static str{self.client.server_name()}
    pub fn supports(&self, path:&Path)->bool{ matches!(path.extension().and_then(|e|e.to_str()).map(|e|e.to_ascii_lowercase()).as_deref(), Some("rs")|Some("ts")|Some("tsx")|Some("js")|Some("jsx")) }
    pub fn rename(&self, uri:&str, position: Position, new_name:&str)->RenameResult{
        let params=RenameParams{text_document:TextDocumentIdentifier{uri:uri.to_string()},position,new_name:new_name.to_string()};
        let req=LspRequest::new(next_request_id(),"textDocument/rename", serde_json::to_value(&params).map_err(|e|RenameError::InvalidResponse(format!("serialize params: {e}")))?);
        let resp=self.client.send(req).map_err(RenameError::ServerError)?;
        if let Some(err)=resp.error{ return Err(RenameError::ServerError(format!("{} (code {})",err.message,err.code))); }
        match resp.result{ None=>Ok(None), Some(ref raw) if raw.is_null()=>Ok(None), Some(ref raw)=> parse_edit(raw).map(Some)}
    }
}
fn parse_edit(raw:&Value)->Result<WorkspaceEdit,RenameError>{
    serde_json::from_value::<WorkspaceEdit>(raw.clone()).map_err(|e|RenameError::InvalidResponse(format!("WorkspaceEdit: {e}")))
}
fn next_request_id()->u64{ use std::sync::atomic::{AtomicU64,Ordering}; static C:AtomicU64=AtomicU64::new(1); C.fetch_add(1,Ordering::Relaxed) }

#[cfg(test)]
mod tests{
    use super::*; use crate::lsp_client::{LspResponse, Position}; use serde_json::json;
    struct FakeOk(Value); struct FakeErr(String);
    impl LspClient for FakeOk{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:Some(self.0.clone()),error:None})} fn server_name(&self)->&'static str{"fake"}}
    impl LspClient for FakeErr{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Err(self.0.clone())} fn server_name(&self)->&'static str{"fake"}}
    fn pos()->Position{Position{line:0,character:5}}
    #[test] fn returns_edit(){ let p=RenameProvider::new(Box::new(FakeOk(json!({"changes":{"file:///a.rs":[{"range":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}},"newText":"bar"}]}})))); let e=p.rename("file:///a.rs",pos(),"bar").unwrap().unwrap(); assert_eq!(e.changes.unwrap().len(),1); }
    #[test] fn null_returns_none(){ let p=RenameProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.rename("file:///a.rs",pos(),"bar").unwrap().is_none()); }
    #[test] fn server_err(){ let p=RenameProvider::new(Box::new(FakeErr("boom".into()))); assert!(matches!(p.rename("file:///a.rs",pos(),"bar"),Err(RenameError::ServerError(_)))); }
    #[test] fn supports(){ let p=RenameProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.supports(std::path::Path::new("foo.rs"))); assert!(!p.supports(std::path::Path::new("foo.md"))); }
    #[test] fn new_name_forwarded(){ #[derive(Clone)] struct Cap; impl LspClient for Cap{fn send(&self,req:LspRequest)->Result<LspResponse,String>{ assert_eq!(req.params.get("newName").and_then(Value::as_str),Some("qux")); Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:Some(Value::Null),error:None})} fn server_name(&self)->&'static str{"fake"}} let p=RenameProvider::new(Box::new(Cap)); p.rename("file:///a.rs",pos(),"qux").unwrap(); }
    #[test] fn lsp_error(){ struct FakeLspErr; impl LspClient for FakeLspErr{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:None,error:Some(crate::lsp_client::LspError{code:-32603,message:"refused".into(),data:None})})} fn server_name(&self)->&'static str{"fake"}} let p=RenameProvider::new(Box::new(FakeLspErr)); assert!(matches!(p.rename("file:///a.rs",pos(),"bar"),Err(RenameError::ServerError(_)))); }
}

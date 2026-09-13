//! `TypeDefinitionProvider` — `textDocument/typeDefinition`.

use std::path::Path;

use serde_json::Value;

use crate::definition::Location;
use crate::lsp_client::{LspClient, LspRequest, Position, TextDocumentIdentifier, TextDocumentPositionParams};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDefinitionError { ServerError(String), InvalidResponse(String) }
impl std::fmt::Display for TypeDefinitionError{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{match self{Self::ServerError(m)=>write!(f,"lsp server error: {m}"), Self::InvalidResponse(m)=>write!(f,"invalid typeDefinition response: {m}")}}}
impl std::error::Error for TypeDefinitionError {}
pub type TypeDefinitionResult = Result<Vec<Location>, TypeDefinitionError>;

pub struct TypeDefinitionProvider{ client: Box<dyn LspClient> }
impl TypeDefinitionProvider{
    pub fn new(c:Box<dyn LspClient>)->Self{Self{client:c}}
    pub fn name(&self)->&'static str{self.client.server_name()}
    pub fn supports(&self,path:&Path)->bool{ matches!(path.extension().and_then(|e|e.to_str()).map(|e|e.to_ascii_lowercase()).as_deref(), Some("rs")|Some("ts")|Some("tsx")|Some("js")|Some("jsx")) }
    pub fn type_definition(&self, uri:&str, position: Position)->TypeDefinitionResult{
        let params=TextDocumentPositionParams{ text_document: TextDocumentIdentifier{uri:uri.to_string()}, position};
        let req=LspRequest::new(next_request_id(),"textDocument/typeDefinition", serde_json::to_value(&params).map_err(|e|TypeDefinitionError::InvalidResponse(format!("serialize: {e}")))?);
        let resp=self.client.send(req).map_err(TypeDefinitionError::ServerError)?;
        if let Some(err)=resp.error{ return Err(TypeDefinitionError::ServerError(format!("{} (code {})",err.message,err.code))); }
        match resp.result{ None=>Ok(Vec::new()), Some(ref raw) if raw.is_null()=>Ok(Vec::new()), Some(ref raw)=> parse_locations(raw)}
    }
}
fn parse_locations(raw:&Value)->TypeDefinitionResult{
    match raw{
        Value::Null=>Ok(Vec::new()),
        Value::Array(_)=>{ let v:Vec<Location>=serde_json::from_value(raw.clone()).map_err(|e|TypeDefinitionError::InvalidResponse(format!("array: {e}")))?; Ok(v) }
        Value::Object(_)=>{ let v:Location=serde_json::from_value(raw.clone()).map_err(|e|TypeDefinitionError::InvalidResponse(format!("single: {e}")))?; Ok(vec![v]) }
        _=> Err(TypeDefinitionError::InvalidResponse(format!("unexpected: {raw}"))),
    }
}
fn next_request_id()->u64{ use std::sync::atomic::{AtomicU64,Ordering}; static C:AtomicU64=AtomicU64::new(1); C.fetch_add(1,Ordering::Relaxed) }

#[cfg(test)]
mod tests{
    use super::*; use crate::lsp_client::{LspResponse,Position}; use serde_json::json;
    struct FakeOk(Value); struct FakeErr(String);
    impl LspClient for FakeOk{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:Some(self.0.clone()),error:None})} fn server_name(&self)->&'static str{"fake"}}
    impl LspClient for FakeErr{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Err(self.0.clone())} fn server_name(&self)->&'static str{"fake"}}
    fn pos()->Position{Position{line:0,character:5}}
    fn loc(u:&str)->Value{json!({"uri":u,"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":5}}})}
    #[test] fn single(){ let p=TypeDefinitionProvider::new(Box::new(FakeOk(loc("file:///a.rs")))); assert_eq!(p.type_definition("file:///a.rs",pos()).unwrap().len(),1); }
    #[test] fn array(){ let p=TypeDefinitionProvider::new(Box::new(FakeOk(json!([loc("file:///a.rs"),loc("file:///b.rs")])))); assert_eq!(p.type_definition("file:///a.rs",pos()).unwrap().len(),2); }
    #[test] fn null_empty(){ let p=TypeDefinitionProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.type_definition("file:///a.rs",pos()).unwrap().is_empty()); }
    #[test] fn server_err(){ let p=TypeDefinitionProvider::new(Box::new(FakeErr("boom".into()))); assert!(matches!(p.type_definition("file:///a.rs",pos()),Err(TypeDefinitionError::ServerError(_)))); }
    #[test] fn supports(){ let p=TypeDefinitionProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.supports(std::path::Path::new("foo.ts"))); assert!(!p.supports(std::path::Path::new("foo.md"))); }
}

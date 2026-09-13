//! `ReferencesProvider` — `textDocument/references`.
//!
//! The LSP spec's references request needs an extra
//! `context: { includeDeclaration: bool }` param; everything else
//! is the same `TextDocumentPositionParams` shape.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::definition::Location;
use crate::lsp_client::{LspClient, LspRequest, Position, TextDocumentIdentifier};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferencesError {
    ServerError(String),
    InvalidResponse(String),
}
impl std::fmt::Display for ReferencesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerError(m) => write!(f, "lsp server error: {m}"),
            Self::InvalidResponse(m) => write!(f, "invalid references response: {m}"),
        }
    }
}
impl std::error::Error for ReferencesError {}
pub type ReferencesResult = Result<Vec<Location>, ReferencesError>;

#[derive(Debug, Clone, Serialize)]
struct ReferencesParams {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentIdentifier,
    position: Position,
    context: ReferencesContext,
}
#[derive(Debug, Clone, Serialize)]
struct ReferencesContext {
    #[serde(rename = "includeDeclaration")]
    include_declaration: bool,
}

pub struct ReferencesProvider {
    client: Box<dyn LspClient>,
}
impl ReferencesProvider {
    pub fn new(client: Box<dyn LspClient>) -> Self { Self { client } }
    pub fn name(&self) -> &'static str { self.client.server_name() }
    pub fn supports(&self, path: &Path) -> bool {
        matches!(path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("rs") | Some("ts") | Some("tsx") | Some("js") | Some("jsx"))
    }
    /// Find all references to the symbol at `position` in `uri`.
    pub fn references(&self, uri: &str, position: Position, include_declaration: bool) -> ReferencesResult {
        let params = ReferencesParams {
            text_document: TextDocumentIdentifier { uri: uri.to_string() },
            position,
            context: ReferencesContext { include_declaration },
        };
        let req = LspRequest::new(
            next_request_id(),
            "textDocument/references",
            serde_json::to_value(&params).map_err(|e| ReferencesError::InvalidResponse(format!("serialize params: {e}")))?,
        );
        let resp = self.client.send(req).map_err(ReferencesError::ServerError)?;
        if let Some(err) = resp.error {
            return Err(ReferencesError::ServerError(format!("{} (code {})", err.message, err.code)));
        }
        match resp.result {
            None => Ok(Vec::new()),
            Some(ref raw) if raw.is_null() => Ok(Vec::new()),
            Some(ref raw) => parse_locations(raw),
        }
    }
}
fn parse_locations(raw: &Value) -> ReferencesResult {
    match raw {
        Value::Null => Ok(Vec::new()),
        Value::Array(_) => {
            let locs: Vec<Location> = serde_json::from_value(raw.clone()).map_err(|e| ReferencesError::InvalidResponse(format!("array of Location: {e}")))?;
            Ok(locs)
        }
        Value::Object(_) => {
            let loc: Location = serde_json::from_value(raw.clone()).map_err(|e| ReferencesError::InvalidResponse(format!("single Location: {e}")))?;
            Ok(vec![loc])
        }
        _ => Err(ReferencesError::InvalidResponse(format!("unexpected payload: {raw}"))),
    }
}
fn next_request_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static C: AtomicU64 = AtomicU64::new(1);
    C.fetch_add(1, Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_client::{LspResponse, Position};
    use serde_json::json;
    struct FakeOk(Value); struct FakeErr(String);
    impl LspClient for FakeOk { fn send(&self, _: LspRequest) -> Result<LspResponse, String> { Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:Some(self.0.clone()),error:None}) } fn server_name(&self)->&'static str{"fake"} }
    impl LspClient for FakeErr { fn send(&self,_:LspRequest)->Result<LspResponse,String>{Err(self.0.clone())} fn server_name(&self)->&'static str{"fake"} }
    fn pos()->Position{Position{line:0,character:5}}
    fn loc(uri:&str)->Value{json!({"uri":uri,"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":5}}})}
    #[test] fn returns_locations(){ let p=ReferencesProvider::new(Box::new(FakeOk(json!([loc("file:///a.rs")])))); assert_eq!(p.references("file:///a.rs",pos(),false).unwrap().len(),1); }
    #[test] fn empty_null(){ let p=ReferencesProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.references("file:///a.rs",pos(),false).unwrap().is_empty()); }
    #[test] fn server_err(){ let p=ReferencesProvider::new(Box::new(FakeErr("boom".into()))); assert!(matches!(p.references("file:///a.rs",pos(),false),Err(ReferencesError::ServerError(_)))); }
    #[test] fn supports(){ let p=ReferencesProvider::new(Box::new(FakeOk(Value::Null))); assert!(p.supports(Path::new("foo.rs"))); assert!(!p.supports(Path::new("foo.md"))); }
    #[test] fn lsp_error_field(){ struct FakeLspErr; impl LspClient for FakeLspErr{fn send(&self,_:LspRequest)->Result<LspResponse,String>{Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:None,error:Some(crate::lsp_client::LspError{code:-32603,message:"no refs".into(),data:None})})} fn server_name(&self)->&'static str{"fake"}} let p=ReferencesProvider::new(Box::new(FakeLspErr)); assert!(matches!(p.references("file:///a.rs",pos(),false),Err(ReferencesError::ServerError(_)))); }
    #[test] fn include_declaration_propagated(){
        #[derive(Clone)] struct Capture; impl LspClient for Capture{fn send(&self,req:LspRequest)->Result<LspResponse,String>{ assert!(req.params.get("context").and_then(|c|c.get("includeDeclaration")).and_then(Value::as_bool)==Some(true)); Ok(LspResponse{jsonrpc:Some("2.0".into()),id:Some(1),result:Some(json!([])),error:None})} fn server_name(&self)->&'static str{"fake"}}
        let p=ReferencesProvider::new(Box::new(Capture)); assert!(p.references("file:///a.rs",pos(),true).unwrap().is_empty());
    }
}

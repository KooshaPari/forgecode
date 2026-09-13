//! `ImplementationProvider` — `textDocument/implementation` (`goto implementation`).
//!
//! Thin wrapper over [`LspClient`], same shape as `definition.rs` /
//! `hover.rs`: serialises `TextDocumentPositionParams` into a
//! `textDocument/implementation` request and normalises the server's
//! `Location | Location[] | null` response into `Vec<Location>`.

use std::path::Path;

use serde_json::Value;

use crate::definition::Location;
use crate::lsp_client::{
    LspClient, LspRequest, Position, TextDocumentIdentifier, TextDocumentPositionParams,
};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImplementationError {
    ServerError(String),
    InvalidResponse(String),
}

impl std::fmt::Display for ImplementationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerError(m) => write!(f, "lsp server error: {m}"),
            Self::InvalidResponse(m) => write!(f, "invalid implementation response: {m}"),
        }
    }
}
impl std::error::Error for ImplementationError {}

pub type ImplementationResult = Result<Vec<Location>, ImplementationError>;

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct ImplementationProvider {
    client: Box<dyn LspClient>,
}

impl ImplementationProvider {
    pub fn new(client: Box<dyn LspClient>) -> Self {
        Self { client }
    }
    pub fn name(&self) -> &'static str {
        self.client.server_name()
    }
    pub fn supports(&self, path: &Path) -> bool {
        matches!(
            path.extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref(),
            Some("rs") | Some("ts") | Some("tsx") | Some("js") | Some("jsx")
        )
    }
    pub fn implementation(&self, uri: &str, position: Position) -> ImplementationResult {
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.to_string() },
            position,
        };
        let req = LspRequest::new(
            next_request_id(),
            "textDocument/implementation",
            serde_json::to_value(&params)
                .map_err(|e| ImplementationError::InvalidResponse(format!("serialize params: {e}")))?,
        );
        let resp = self.client.send(req).map_err(ImplementationError::ServerError)?;
        if let Some(err) = resp.error {
            return Err(ImplementationError::ServerError(format!(
                "{} (code {})",
                err.message, err.code
            )));
        }
        match resp.result {
            None => Ok(Vec::new()),
            Some(ref raw) if raw.is_null() => Ok(Vec::new()),
            Some(ref raw) => parse_locations(raw),
        }
    }
}

fn parse_locations(raw: &Value) -> ImplementationResult {
    match raw {
        Value::Null => Ok(Vec::new()),
        Value::Array(_) => {
            let locs: Vec<Location> = serde_json::from_value(raw.clone())
                .map_err(|e| ImplementationError::InvalidResponse(format!("array of Location: {e}")))?;
            Ok(locs)
        }
        Value::Object(_) => {
            let loc: Location = serde_json::from_value(raw.clone())
                .map_err(|e| ImplementationError::InvalidResponse(format!("single Location: {e}")))?;
            Ok(vec![loc])
        }
        _ => Err(ImplementationError::InvalidResponse(format!(
            "unexpected payload kind: {raw}"
        ))),
    }
}

fn next_request_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_client::{LspResponse, Position, Range};
    use serde_json::json;

    struct FakeOk(Value);
    struct FakeErr(String);
    impl LspClient for FakeOk {
        fn send(&self, _: LspRequest) -> Result<LspResponse, String> { Ok(LspResponse { jsonrpc: Some("2.0".into()), id: Some(1), result: Some(self.0.clone()), error: None }) }
        fn server_name(&self) -> &'static str { "fake" }
    }
    impl LspClient for FakeErr {
        fn send(&self, _: LspRequest) -> Result<LspResponse, String> { Err(self.0.clone()) }
        fn server_name(&self) -> &'static str { "fake" }
    }
    fn pos() -> Position { Position { line: 0, character: 5 } }
    fn loc(uri: &str) -> Value { json!({"uri": uri, "range": {"start": {"line": 1, "character": 0}, "end": {"line": 1, "character": 5}}}) }

    #[test] fn single_location() {
        let p = ImplementationProvider::new(Box::new(FakeOk(loc("file:///a.rs"))));
        assert_eq!(p.implementation("file:///a.rs", pos()).unwrap().len(), 1);
    }
    #[test] fn array_locations() {
        let p = ImplementationProvider::new(Box::new(FakeOk(json!([loc("file:///a.rs"), loc("file:///b.rs")]))));
        assert_eq!(p.implementation("file:///a.rs", pos()).unwrap().len(), 2);
    }
    #[test] fn null_returns_empty() {
        let p = ImplementationProvider::new(Box::new(FakeOk(Value::Null)));
        assert!(p.implementation("file:///a.rs", pos()).unwrap().is_empty());
    }
    #[test] fn server_error_propagated() {
        let p = ImplementationProvider::new(Box::new(FakeErr("boom".into())));
        assert!(matches!(p.implementation("file:///a.rs", pos()), Err(ImplementationError::ServerError(_))));
    }
    #[test] fn supports_rs_and_ts() {
        let p = ImplementationProvider::new(Box::new(FakeOk(Value::Null)));
        assert!(p.supports(Path::new("foo.rs")));
        assert!(p.supports(Path::new("foo.ts")));
        assert!(!p.supports(Path::new("foo.md")));
    }
    #[test] fn invalid_payload_returns_error() {
        let p = ImplementationProvider::new(Box::new(FakeOk(json!(42))));
        assert!(matches!(p.implementation("file:///a.rs", pos()), Err(ImplementationError::InvalidResponse(_))));
    }
    #[test] fn lsp_error_field_returned() {
        struct FakeLspErr;
        impl LspClient for FakeLspErr {
            fn send(&self, _: LspRequest) -> Result<LspResponse, String> {
                Ok(LspResponse { jsonrpc: Some("2.0".into()), id: Some(1), result: None, error: Some(crate::lsp_client::LspError { code: -32603, message: "no impl".into(), data: None }) })
            }
            fn server_name(&self) -> &'static str { "fake" }
        }
        let p = ImplementationProvider::new(Box::new(FakeLspErr));
        assert!(matches!(p.implementation("file:///a.rs", pos()), Err(ImplementationError::ServerError(_))));
    }
}

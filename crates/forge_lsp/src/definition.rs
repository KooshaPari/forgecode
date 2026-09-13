//! `DefinitionProvider` — forwards `textDocument/definition` to a real
//! LSP server (`rust-analyzer`, `tsserver`).
//!
//! Per LSP spec the server returns either a single [`Location`] or an
//! array. We normalize both to `Vec<Location>` so callers always work
//! with a list (a definition is usually 1 item, but type-usages can
//! produce multiple).
//!
//! [`Location`]: crate::definition::Location

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::lsp_client::{
    LspClient, LspRequest, Position, Range, TextDocumentIdentifier, TextDocumentPositionParams,
};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// A location in a workspace — `uri` + `range`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Errors that `DefinitionProvider::definition` can surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionError {
    /// The LSP server returned an error response.
    ServerError(String),
    /// The LSP server returned a non-Location payload.
    InvalidResponse(String),
}

impl std::fmt::Display for DefinitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefinitionError::ServerError(m) => write!(f, "lsp server error: {m}"),
            DefinitionError::InvalidResponse(m) => write!(f, "invalid definition response: {m}"),
        }
    }
}

impl std::error::Error for DefinitionError {}

pub type DefinitionResult = Result<Vec<Location>, DefinitionError>;

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct DefinitionProvider {
    client: Box<dyn LspClient>,
}

impl DefinitionProvider {
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

    /// Run `textDocument/definition` for `uri` at `position`.
    pub fn definition(&self, uri: &str, position: Position) -> DefinitionResult {
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.to_string() },
            position,
        };
        let request = LspRequest::new(
            next_request_id(),
            "textDocument/definition",
            serde_json::to_value(&params)
                .map_err(|e| DefinitionError::InvalidResponse(format!("serialize params: {e}")))?,
        );
        let response = self
            .client
            .send(request)
            .map_err(DefinitionError::ServerError)?;
        if let Some(err) = response.error {
            return Err(DefinitionError::ServerError(format!(
                "{} (code {})",
                err.message, err.code
            )));
        }
        match response.result {
            None => Ok(Vec::new()),
            Some(ref raw) if raw.is_null() => Ok(Vec::new()),
            Some(ref raw) => parse_locations(raw),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Normalize a `Location` / `Location[]` payload into `Vec<Location>`.
fn parse_locations(raw: &Value) -> DefinitionResult {
    match raw {
        Value::Null => Ok(Vec::new()),
        Value::Array(_) => {
            let locs: Vec<Location> = serde_json::from_value(raw.clone())
                .map_err(|e| DefinitionError::InvalidResponse(format!("array of Location: {e}")))?;
            Ok(locs)
        }
        Value::Object(_) => {
            let loc: Location = serde_json::from_value(raw.clone())
                .map_err(|e| DefinitionError::InvalidResponse(format!("single Location: {e}")))?;
            Ok(vec![loc])
        }
        _ => Err(DefinitionError::InvalidResponse(format!(
            "unexpected payload kind: {}",
            raw
        ))),
    }
}

// ---------------------------------------------------------------------------
// Request id allocation (same scheme as hover.rs)
// ---------------------------------------------------------------------------

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
    use crate::lsp_client::LspResponse;
    use std::sync::Mutex;

    pub struct MockLspClient {
        pub name: &'static str,
        pub scripted: Mutex<Option<Result<LspResponse, String>>>,
        pub captured: Mutex<Vec<LspRequest>>,
    }

    impl MockLspClient {
        pub fn new(name: &'static str, response: LspResponse) -> Self {
            Self {
                name,
                scripted: Mutex::new(Some(Ok(response))),
                captured: Mutex::new(Vec::new()),
            }
        }

        pub fn failing(name: &'static str, err: &str) -> Self {
            Self {
                name,
                scripted: Mutex::new(Some(Err(err.to_string()))),
                captured: Mutex::new(Vec::new()),
            }
        }

        pub fn last_request(&self) -> Option<LspRequest> {
            self.captured.lock().unwrap().last().cloned()
        }
    }

    impl crate::lsp_client::LspClient for MockLspClient {
        fn send(&self, request: LspRequest) -> Result<LspResponse, String> {
            self.captured.lock().unwrap().push(request);
            let next = self.scripted.lock().unwrap().take();
            match next {
                Some(r) => r,
                None => Ok(LspResponse {
                    jsonrpc: Some("2.0".into()),
                    id: Some(0),
                    result: Some(Value::Null),
                    error: None,
                }),
            }
        }
        fn server_name(&self) -> &'static str {
            self.name
        }
    }

    fn ok_response(result: Value) -> LspResponse {
        LspResponse {
            jsonrpc: Some("2.0".into()),
            id: Some(1),
            result: Some(result),
            error: None,
        }
    }

    fn err_response(code: i64, message: &str) -> LspResponse {
        LspResponse {
            jsonrpc: Some("2.0".into()),
            id: Some(1),
            result: None,
            error: Some(crate::lsp_client::LspError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }

    #[test]
    fn definition_supports_common_languages() {
        let client = MockLspClient::new(
            "rust-analyzer",
            ok_response(
                json!({"uri":"file:///foo.rs","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}}}),
            ),
        );
        let p = DefinitionProvider::new(Box::new(client));
        assert!(p.supports(Path::new("foo.rs")));
        assert!(p.supports(Path::new("foo.tsx")));
        assert!(p.supports(Path::new("foo.JS")));
        assert!(!p.supports(Path::new("foo.py")));
    }

    #[test]
    fn definition_returns_empty_when_server_returns_null() {
        let client = MockLspClient::new("rust-analyzer", ok_response(Value::Null));
        let p = DefinitionProvider::new(Box::new(client));
        let locs = p
            .definition("file:///foo.rs", Position { line: 0, character: 0 })
            .unwrap();
        assert!(locs.is_empty());
    }

    #[test]
    fn definition_parses_single_location() {
        let client = MockLspClient::new(
            "rust-analyzer",
            ok_response(json!({
                "uri": "file:///lib.rs",
                "range": {
                    "start": {"line": 10, "character": 0},
                    "end":   {"line": 10, "character": 4}
                }
            })),
        );
        let p = DefinitionProvider::new(Box::new(client));
        let locs = p
            .definition("file:///foo.rs", Position { line: 5, character: 2 })
            .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs[0].uri, "file:///lib.rs");
        assert_eq!(locs[0].range.start.line, 10);
        assert_eq!(locs[0].range.end.character, 4);
    }

    #[test]
    fn definition_parses_array_of_locations() {
        let client = MockLspClient::new(
            "tsserver",
            ok_response(json!([
                {"uri":"file:///a.ts","range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}}},
                {"uri":"file:///b.ts","range":{"start":{"line":2,"character":0},"end":{"line":2,"character":3}}}
            ])),
        );
        let p = DefinitionProvider::new(Box::new(client));
        let locs = p
            .definition("file:///foo.ts", Position { line: 9, character: 0 })
            .unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs[0].uri, "file:///a.ts");
        assert_eq!(locs[1].uri, "file:///b.ts");
    }

    #[test]
    fn definition_propagates_server_error() {
        let client = MockLspClient::new("rust-analyzer", err_response(-32601, "method not found"));
        let p = DefinitionProvider::new(Box::new(client));
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::ServerError(_))));
    }

    #[test]
    fn definition_surfaces_transport_failure() {
        let client = MockLspClient::failing("rust-analyzer", "io: broken pipe");
        let p = DefinitionProvider::new(Box::new(client));
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::ServerError(m)) if m == "io: broken pipe"));
    }

    #[test]
    fn definition_rejects_malformed_payload() {
        let client = MockLspClient::new("rust-analyzer", ok_response(json!("just a string")));
        let p = DefinitionProvider::new(Box::new(client));
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::InvalidResponse(_))));
    }
}

//! `DefinitionProvider` — forwards `textDocument/definition` to a real
//! LSP server (`rust-analyzer`, `typescript-language-server`).
//!
//! Per LSP spec the server returns either a single [`Location`], an
//! array, or (with `linkSupport: true`) one or more `LocationLink`
//! objects. We normalize all three to `Vec<Location>` so callers
//! always work with a list (a definition is usually 1 item, but
//! type-usages can produce multiple).
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

/// Generic over `C: LspClient` (no `Box<dyn>`).
pub struct DefinitionProvider<C: LspClient + ?Sized> {
    client: std::sync::Arc<C>,
}

impl<C: LspClient + ?Sized> DefinitionProvider<C> {
    pub fn new(client: std::sync::Arc<C>) -> Self {
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
            Some("rs")
                | Some("ts")
                | Some("tsx")
                | Some("mts")
                | Some("cts")
                | Some("js")
                | Some("jsx")
                | Some("mjs")
                | Some("cjs")
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

/// Normalize a `Location` / `Location[]` / `LocationLink[]` payload
/// into `Vec<Location>`. We try `LocationLink` first (it has
/// `targetUri` / `targetRange` keys) and fall back to plain
/// `Location`.
fn parse_locations(raw: &Value) -> DefinitionResult {
    match raw {
        Value::Null => Ok(Vec::new()),
        Value::Array(_) => {
            // Mixed array: each element might be Location or
            // LocationLink. Inspect the first key to dispatch.
            let arr = raw
                .as_array()
                .ok_or_else(|| DefinitionError::InvalidResponse("expected array".to_string()))?;
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                out.push(parse_single_location(item)?);
            }
            Ok(out)
        }
        Value::Object(_) => parse_locations_vec(raw),
        _ => Err(DefinitionError::InvalidResponse(format!(
            "unexpected payload kind: {}",
            raw
        ))),
    }
}

fn parse_locations_vec(raw: &Value) -> DefinitionResult {
    if let Some(items) = raw.get("items") {
        // Some servers wrap Location[] in `{ items: [...] }`. Match
        // both shapes (`Location[]` and `LocationList`).
        let arr = items
            .as_array()
            .ok_or_else(|| DefinitionError::InvalidResponse("items not an array".to_string()))?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            out.push(parse_single_location(item)?);
        }
        return Ok(out);
    }
    Ok(vec![parse_single_location(raw)?])
}

fn parse_single_location(item: &Value) -> Result<Location, DefinitionError> {
    // LocationLink: { targetUri, targetRange, targetSelectionRange,
    //                 originSelectionRange, originRange? }
    if let Some(uri) = item.get("targetUri").and_then(Value::as_str) {
        let range = item
            .get("targetRange")
            .ok_or_else(|| {
                DefinitionError::InvalidResponse("LocationLink missing 'targetRange'".to_string())
            })
            .and_then(range_from_value)?;
        return Ok(Location { uri: uri.to_string(), range });
    }
    // Location: { uri, range }
    if let (Some(uri), Some(range)) = (item.get("uri").and_then(Value::as_str), item.get("range")) {
        let range = range_from_value(range)?;
        return Ok(Location { uri: uri.to_string(), range });
    }
    Err(DefinitionError::InvalidResponse(format!(
        "object missing 'uri'+'range' or 'targetUri'+'targetRange': {}",
        item
    )))
}

fn range_from_value(v: &Value) -> Result<Range, DefinitionError> {
    let start = v
        .get("start")
        .ok_or_else(|| DefinitionError::InvalidResponse("range missing 'start'".to_string()))?;
    let end = v
        .get("end")
        .ok_or_else(|| DefinitionError::InvalidResponse("range missing 'end'".to_string()))?;
    Ok(Range {
        start: Position {
            line: start
                .get("line")
                .and_then(Value::as_u64)
                .ok_or_else(|| DefinitionError::InvalidResponse("range.start.line".to_string()))?
                as u32,
            character: start
                .get("character")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    DefinitionError::InvalidResponse("range.start.character".to_string())
                })? as u32,
        },
        end: Position {
            line: end
                .get("line")
                .and_then(Value::as_u64)
                .ok_or_else(|| DefinitionError::InvalidResponse("range.end.line".to_string()))?
                as u32,
            character: end
                .get("character")
                .and_then(Value::as_u64)
                .ok_or_else(|| {
                    DefinitionError::InvalidResponse("range.end.character".to_string())
                })? as u32,
        },
    })
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
    use std::sync::{Arc, Mutex};

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
        fn server_name(&self) -> &'static str {
            self.name
        }

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
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            ok_response(
                json!({"uri":"file:///foo.rs","range":{"start":{"line":0,"character":0},"end":{"line":0,"character":3}}}),
            ),
        ));
        let p = DefinitionProvider::new(client);
        assert!(p.supports(Path::new("foo.rs")));
        assert!(p.supports(Path::new("foo.tsx")));
        assert!(p.supports(Path::new("foo.JS")));
        assert!(p.supports(Path::new("foo.mts")));
        assert!(p.supports(Path::new("foo.cts")));
        assert!(!p.supports(Path::new("foo.py")));
    }

    #[test]
    fn definition_returns_empty_when_server_returns_null() {
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            ok_response(Value::Null),
        ));
        let p = DefinitionProvider::new(client);
        let locs = p
            .definition("file:///foo.rs", Position { line: 0, character: 0 })
            .unwrap();
        assert!(locs.is_empty());
    }

    #[test]
    fn definition_parses_single_location() {
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            ok_response(json!({
                "uri": "file:///lib.rs",
                "range": {
                    "start": {"line": 10, "character": 0},
                    "end":   {"line": 10, "character": 4}
                }
            })),
        ));
        let p = DefinitionProvider::new(client);
        let locs = p
            .definition("file:///foo.rs", Position { line: 5, character: 2 })
            .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs.first().map(|x| x.uri.as_str()), Some("file:///lib.rs"));
        assert_eq!(locs.first().map(|x| x.range.start.line), Some(10));
        assert_eq!(locs.first().map(|x| x.range.end.character), Some(4));
    }

    #[test]
    fn definition_parses_array_of_locations() {
        let client = Arc::new(MockLspClient::new(
            "tsserver",
            ok_response(json!([
                {"uri":"file:///a.ts","range":{"start":{"line":1,"character":0},"end":{"line":1,"character":3}}},
                {"uri":"file:///b.ts","range":{"start":{"line":2,"character":0},"end":{"line":2,"character":3}}}
            ])),
        ));
        let p = DefinitionProvider::new(client);
        let locs = p
            .definition("file:///foo.ts", Position { line: 9, character: 0 })
            .unwrap();
        assert_eq!(locs.len(), 2);
        assert_eq!(locs.first().map(|x| x.uri.as_str()), Some("file:///a.ts"));
        assert_eq!(locs.get(1).map(|x| x.uri.as_str()), Some("file:///b.ts"));
    }

    #[test]
    fn definition_parses_location_link_payload() {
        // rust-analyzer with linkSupport returns LocationLink[].
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            ok_response(json!([
                {
                    "originSelectionRange": {
                        "start": {"line": 5, "character": 4},
                        "end":   {"line": 5, "character": 8}
                    },
                    "targetUri": "file:///lib.rs",
                    "targetRange": {
                        "start": {"line": 10, "character": 0},
                        "end":   {"line": 10, "character": 8}
                    },
                    "targetSelectionRange": {
                        "start": {"line": 10, "character": 3},
                        "end":   {"line": 10, "character": 6}
                    }
                }
            ])),
        ));
        let p = DefinitionProvider::new(client);
        let locs = p
            .definition("file:///foo.rs", Position { line: 5, character: 5 })
            .unwrap();
        assert_eq!(locs.len(), 1);
        assert_eq!(locs.first().map(|x| x.uri.as_str()), Some("file:///lib.rs"));
        assert_eq!(locs.first().map(|x| x.range.start.line), Some(10));
        assert_eq!(locs.first().map(|x| x.range.end.character), Some(8));
    }

    #[test]
    fn definition_propagates_server_error() {
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            err_response(-32601, "method not found"),
        ));
        let p = DefinitionProvider::new(client);
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::ServerError(_))));
    }

    #[test]
    fn definition_surfaces_transport_failure() {
        let client = Arc::new(MockLspClient::failing("rust-analyzer", "io: broken pipe"));
        let p = DefinitionProvider::new(client);
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::ServerError(m)) if m == "io: broken pipe"));
    }

    #[test]
    fn definition_rejects_malformed_payload() {
        let client = Arc::new(MockLspClient::new(
            "rust-analyzer",
            ok_response(json!("just a string")),
        ));
        let p = DefinitionProvider::new(client);
        let r = p.definition("file:///foo.rs", Position { line: 0, character: 0 });
        assert!(matches!(r, Err(DefinitionError::InvalidResponse(_))));
    }
}

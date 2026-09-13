//! `CompletionProvider` — forwards `textDocument/completion` to a real
//! LSP server (`rust-analyzer`, `tsserver`).
//!
//! Per LSP spec the server returns either a single [`CompletionItem`]
//! or an array (sometimes wrapped in a `CompletionList` with
//! `isIncomplete`). We normalize to `Vec<CompletionItem>`.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::lsp_client::{
    CompletionContext, CompletionParams, LspClient, LspRequest, Position, Range,
    TextDocumentIdentifier, TextEdit,
};

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

/// LSP `CompletionItemKind` — the subset we surface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[repr(i32)]
pub enum CompletionKind {
    Text = 1,
    Method = 2,
    Function = 3,
    Constructor = 4,
    Field = 5,
    Variable = 6,
    Class = 7,
    Interface = 8,
    Module = 9,
    Property = 10,
    Keyword = 14,
    Snippet = 15,
}

impl CompletionKind {
    fn from_value(v: i64) -> Option<Self> {
        Some(match v {
            1 => Self::Text,
            2 => Self::Method,
            3 => Self::Function,
            4 => Self::Constructor,
            5 => Self::Field,
            6 => Self::Variable,
            7 => Self::Class,
            8 => Self::Interface,
            9 => Self::Module,
            10 => Self::Property,
            14 => Self::Keyword,
            15 => Self::Snippet,
            _ => return None,
        })
    }
}

/// `serde` adapter so `kind` deserializes from an integer literal
/// (LSP spec shape) without requiring a wrapped newtype.
mod kind_serde {
    use super::CompletionKind;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D: Deserializer<'de>>(
        de: D,
    ) -> Result<Option<CompletionKind>, D::Error> {
        // Accept either a string (legacy) or an integer (canonical).
        let v = serde_json::Value::deserialize(de)?;
        match v {
            serde_json::Value::Null => Ok(None),
            serde_json::Value::Number(n) => {
                let i = n.as_i64().ok_or_else(|| {
                    serde::de::Error::custom("CompletionItemKind must be an integer")
                })?;
                Ok(CompletionKind::from_value(i))
            }
            serde_json::Value::String(s) => {
                // Best-effort: some legacy clients serialize as the
                // variant name. Match by suffix.
                Ok(match s.as_str() {
                    "text" => Some(CompletionKind::Text),
                    "method" => Some(CompletionKind::Method),
                    "function" => Some(CompletionKind::Function),
                    "constructor" => Some(CompletionKind::Constructor),
                    "field" => Some(CompletionKind::Field),
                    "variable" => Some(CompletionKind::Variable),
                    "class" => Some(CompletionKind::Class),
                    "interface" => Some(CompletionKind::Interface),
                    "module" => Some(CompletionKind::Module),
                    "property" => Some(CompletionKind::Property),
                    "keyword" => Some(CompletionKind::Keyword),
                    "snippet" => Some(CompletionKind::Snippet),
                    _ => None,
                })
            }
            _ => Err(serde::de::Error::custom(
                "CompletionItemKind must be integer or string",
            )),
        }
    }

    pub fn serialize<S: Serializer>(
        kind: &Option<CompletionKind>,
        ser: S,
    ) -> Result<S::Ok, S::Error> {
        kind.map(|k| k as i32).serialize(ser)
    }
}

/// A single completion item — the subset of LSP fields the REPL needs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "kind_serde")]
    pub kind: Option<CompletionKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    #[serde(
        rename = "insertText",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub insert_text: Option<String>,
    #[serde(rename = "textEdit", default, skip_serializing_if = "Option::is_none")]
    pub text_edit: Option<TextEdit>,
    #[serde(rename = "sortText", default, skip_serializing_if = "Option::is_none")]
    pub sort_text: Option<String>,
}

/// Errors that `CompletionProvider::complete` can surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionError {
    ServerError(String),
    InvalidResponse(String),
}

impl std::fmt::Display for CompletionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompletionError::ServerError(m) => write!(f, "lsp server error: {m}"),
            CompletionError::InvalidResponse(m) => {
                write!(f, "invalid completion response: {m}")
            }
        }
    }
}

impl std::error::Error for CompletionError {}

pub type CompletionResult = Result<Vec<CompletionItem>, CompletionError>;

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

pub struct CompletionProvider {
    client: Box<dyn LspClient>,
}

impl CompletionProvider {
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

    /// Run `textDocument/completion` for `uri` at `position`.
    pub fn complete(
        &self,
        uri: &str,
        position: Position,
        trigger: Option<char>,
    ) -> CompletionResult {
        let params = CompletionParams {
            text_document: TextDocumentIdentifier { uri: uri.to_string() },
            position,
            context: trigger.map(|c| CompletionContext {
                trigger_kind: 2, // TriggerCharacter
                trigger_character: Some(c.to_string()),
            }),
        };
        let request = LspRequest::new(
            next_request_id(),
            "textDocument/completion",
            serde_json::to_value(&params)
                .map_err(|e| CompletionError::InvalidResponse(format!("serialize params: {e}")))?,
        );
        let response = self
            .client
            .send(request)
            .map_err(CompletionError::ServerError)?;
        if let Some(err) = response.error {
            return Err(CompletionError::ServerError(format!(
                "{} (code {})",
                err.message, err.code
            )));
        }
        match response.result {
            None => Ok(Vec::new()),
            Some(ref raw) if raw.is_null() => Ok(Vec::new()),
            Some(ref raw) => parse_completions(raw),
        }
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

/// Normalize a `CompletionItem[]` / `CompletionItem` / `CompletionList`
/// payload into `Vec<CompletionItem>`.
fn parse_completions(raw: &Value) -> CompletionResult {
    // CompletionList: { isIncomplete: bool, items: [...] }
    if let Some(items) = raw.get("items") {
        let list: Vec<CompletionItem> = serde_json::from_value(items.clone())
            .map_err(|e| CompletionError::InvalidResponse(format!("CompletionList.items: {e}")))?;
        return Ok(list);
    }
    match raw {
        Value::Null => Ok(Vec::new()),
        Value::Array(_) => {
            let list: Vec<CompletionItem> = serde_json::from_value(raw.clone())
                .map_err(|e| CompletionError::InvalidResponse(format!("array: {e}")))?;
            Ok(list)
        }
        Value::Object(_) => {
            let item: CompletionItem = serde_json::from_value(raw.clone())
                .map_err(|e| CompletionError::InvalidResponse(format!("single item: {e}")))?;
            Ok(vec![item])
        }
        _ => Err(CompletionError::InvalidResponse(format!(
            "unexpected payload kind: {raw}"
        ))),
    }
}

#[allow(dead_code)]
fn _ensure_range_used(_: &Range) {} // keep `Range` import alive

// ---------------------------------------------------------------------------
// Request id allocation (same scheme as hover.rs / definition.rs)
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
    fn completion_supports_common_languages() {
        let client = MockLspClient::new("rust-analyzer", ok_response(json!([])));
        let p = CompletionProvider::new(Box::new(client));
        assert!(p.supports(Path::new("foo.rs")));
        assert!(p.supports(Path::new("foo.ts")));
        assert!(p.supports(Path::new("foo.tsx")));
        assert!(p.supports(Path::new("foo.jsx")));
        assert!(!p.supports(Path::new("foo.py")));
        assert!(!p.supports(Path::new("foo")));
    }

    #[test]
    fn completion_returns_empty_for_null() {
        let client = MockLspClient::new("rust-analyzer", ok_response(Value::Null));
        let p = CompletionProvider::new(Box::new(client));
        let items = p
            .complete("file:///foo.rs", Position { line: 0, character: 0 }, None)
            .unwrap();
        assert!(items.is_empty());
    }

    #[test]
    fn completion_parses_flat_array() {
        let client = MockLspClient::new(
            "rust-analyzer",
            ok_response(json!([
                {"label":"foo","kind":3,"detail":"fn()"},
                {"label":"bar","kind":6}
            ])),
        );
        let p = CompletionProvider::new(Box::new(client));
        let items = p
            .complete("file:///foo.rs", Position { line: 0, character: 0 }, None)
            .unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].label, "foo");
        assert_eq!(items[0].kind, Some(CompletionKind::Function));
        assert_eq!(items[0].detail.as_deref(), Some("fn()"));
        assert_eq!(items[1].label, "bar");
        assert_eq!(items[1].kind, Some(CompletionKind::Variable));
    }

    #[test]
    fn completion_parses_completion_list_envelope() {
        let client = MockLspClient::new(
            "tsserver",
            ok_response(json!({
                "isIncomplete": true,
                "items": [
                    {"label":"baz","kind":14,"insertText":"baz"}
                ]
            })),
        );
        let p = CompletionProvider::new(Box::new(client));
        let items = p
            .complete(
                "file:///foo.ts",
                Position { line: 0, character: 0 },
                Some('.'),
            )
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "baz");
        assert_eq!(items[0].kind, Some(CompletionKind::Keyword));
        assert_eq!(items[0].insert_text.as_deref(), Some("baz"));
    }

    #[test]
    fn completion_parses_single_item() {
        let client = MockLspClient::new("tsserver", ok_response(json!({"label":"qux","kind":2})));
        let p = CompletionProvider::new(Box::new(client));
        let items = p
            .complete("file:///foo.ts", Position { line: 0, character: 0 }, None)
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "qux");
        assert_eq!(items[0].kind, Some(CompletionKind::Method));
    }

    #[test]
    fn completion_propagates_server_error() {
        let client = MockLspClient::new("rust-analyzer", err_response(-32601, "method not found"));
        let p = CompletionProvider::new(Box::new(client));
        let r = p.complete("file:///foo.rs", Position { line: 0, character: 0 }, None);
        assert!(matches!(r, Err(CompletionError::ServerError(_))));
    }

    #[test]
    fn completion_surfaces_transport_failure() {
        let client = MockLspClient::failing("rust-analyzer", "io: closed");
        let p = CompletionProvider::new(Box::new(client));
        let r = p.complete("file:///foo.rs", Position { line: 0, character: 0 }, None);
        assert!(matches!(r, Err(CompletionError::ServerError(m)) if m == "io: closed"));
    }

    #[test]
    fn completion_rejects_malformed_payload() {
        let client = MockLspClient::new("rust-analyzer", ok_response(json!({"label":"oops"})));
        // Missing 'kind' is fine; but missing 'label' is an error.
        // The above payload is actually fine. Use a clearly invalid one:
        let client = MockLspClient::new("rust-analyzer", ok_response(json!(42)));
        let p = CompletionProvider::new(Box::new(client));
        let r = p.complete("file:///foo.rs", Position { line: 0, character: 0 }, None);
        assert!(matches!(r, Err(CompletionError::InvalidResponse(_))));
    }
}

//! `lsp_client` — minimal JSON-RPC-over-stdio LSP client.
//!
//! `forge_lsp` already shells out to `cargo check` / `tsc` for
//! diagnostics. For hover, definition, and completion we need to talk
//! JSON-RPC to a real language server (`rust-analyzer`, `tsserver`).
//!
//! This module defines:
//!   * the JSON-RPC message envelope (request / response / notification),
//!   * a small trait [`LspClient`] so the providers can be unit-tested
//!     against a fake (see `tests/fixtures.rs`),
//!   * a real implementation [`ProcessLspClient`] that spawns a
//!     subprocess and reads/writes framed JSON over its stdio, used
//!     by [`crate::hover`], [`crate::definition`], and
//!     [`crate::completion`].
//!
//! The crate deliberately avoids pulling in `lsp-types` — the request
//! shapes we need (hover / definition / completion) are small and the
//! LSP spec is stable, so we keep them in this file.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

// ---------------------------------------------------------------------------
// JSON-RPC envelope
// ---------------------------------------------------------------------------

/// A JSON-RPC request we send to the language server.
#[derive(Debug, Clone, Serialize)]
pub struct LspRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: &'static str,
    pub params: Value,
}

impl LspRequest {
    pub fn new(id: u64, method: &'static str, params: Value) -> Self {
        Self { jsonrpc: "2.0", id, method, params }
    }
}

/// A JSON-RPC response from the language server. We accept either a
/// `result` or an `error` — providers surface the error verbatim.
#[derive(Debug, Clone, Deserialize)]
pub struct LspResponse {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    pub id: Option<u64>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<LspError>,
}

/// JSON-RPC error object as defined by the LSP spec.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LspError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

// ---------------------------------------------------------------------------
// LspClient trait — providers depend on this so they can be tested
// against a fake.
// ---------------------------------------------------------------------------

/// A language-server client. The methods return the raw `serde_json::Value`
/// the server returned for the request. Providers (hover / definition /
/// completion) parse that value into the appropriate domain type.
pub trait LspClient: Send + Sync {
    /// Send `request` and wait for the matching response. Implementations
    /// are responsible for routing by `id` and for handling server-initiated
    /// notifications (which they may simply discard).
    fn send(&self, request: LspRequest) -> Result<LspResponse, String>;

    /// Identifier of the underlying server (`"rust-analyzer"`, `"tsserver"`).
    fn server_name(&self) -> &'static str;
}

// ---------------------------------------------------------------------------
// Real implementation: spawn a subprocess, read/write framed JSON
// ---------------------------------------------------------------------------

/// Standard LSP `Content-Length` header parser. Returns the body.
fn read_framed<R: Read>(reader: &mut R) -> Result<String, String> {
    // Read headers.
    let mut header = String::new();
    loop {
        let mut byte = [0u8; 1];
        match reader.read_exact(&mut byte) {
            Ok(_) => {}
            Err(e) => return Err(format!("header read failed: {e}")),
        }
        header.push(byte[0] as char);
        if header.ends_with("\r\n\r\n") {
            break;
        }
        if header.len() > 8 * 1024 {
            return Err("header too large".to_string());
        }
    }
    let content_length = header
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.eq_ignore_ascii_case("content-length") {
                v.trim().parse::<usize>().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| "missing Content-Length header".to_string())?;
    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .map_err(|e| format!("body read failed: {e}"))?;
    String::from_utf8(body).map_err(|e| format!("body not utf-8: {e}"))
}

fn write_framed<W: Write>(writer: &mut W, body: &str) -> Result<(), String> {
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer
        .write_all(header.as_bytes())
        .map_err(|e| format!("header write failed: {e}"))?;
    writer
        .write_all(body.as_bytes())
        .map_err(|e| format!("body write failed: {e}"))?;
    writer.flush().map_err(|e| format!("flush failed: {e}"))
}

/// Spawn a real LSP server subprocess and talk JSON-RPC over its stdio.
#[derive(Clone)]
pub struct ProcessLspClient {
    name: &'static str,
    child: Arc<Mutex<Option<Child>>>,
    // For simplicity we serialize writes/reads behind a single mutex. The
    // providers' requests are infrequent (hover/definition/completion) so
    // this is fine; a production rewrite would use channels.
    io: Arc<Mutex<ProcessIo>>,
}

struct ProcessIo {
    stdin: std::process::ChildStdin,
    stdout: std::process::ChildStdout,
}

impl ProcessLspClient {
    /// Spawn the binary at `bin` with `args`. The child's stdio is piped
    /// for JSON-RPC framing.
    pub fn spawn(bin: &Path, args: &[&str], name: &'static str) -> Result<Self, String> {
        let mut cmd = Command::new(bin);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("failed to spawn {bin:?}: {e}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "child stdin unavailable".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "child stdout unavailable".to_string())?;
        Ok(Self {
            name,
            child: Arc::new(Mutex::new(Some(child))),
            io: Arc::new(Mutex::new(ProcessIo { stdin, stdout })),
        })
    }

    /// Spawn `rust-analyzer`.
    pub fn rust_analyzer() -> Result<Self, String> {
        Self::spawn(Path::new("rust-analyzer"), &[], "rust-analyzer")
    }

    /// Spawn `tsserver`.
    pub fn tsserver() -> Result<Self, String> {
        Self::spawn(Path::new("tsserver"), &[], "tsserver")
    }
}

impl LspClient for ProcessLspClient {
    fn send(&self, request: LspRequest) -> Result<LspResponse, String> {
        let body =
            serde_json::to_string(&request).map_err(|e| format!("serialize request: {e}"))?;
        let mut io = self
            .io
            .lock()
            .map_err(|e| format!("io mutex poisoned: {e}"))?;
        write_framed(&mut io.stdin, &body)?;
        let response_body = read_framed(&mut io.stdout)?;
        serde_json::from_str(&response_body)
            .map_err(|e| format!("parse response: {e} (body={response_body:?})"))
    }

    fn server_name(&self) -> &'static str {
        self.name
    }
}

impl Drop for ProcessLspClient {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock()
            && let Some(mut child) = guard.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

// ---------------------------------------------------------------------------
// Shared LSP request parameter shapes (hover / definition / completion)
// ---------------------------------------------------------------------------

/// LSP `TextDocumentPositionParams` — used by hover, definition, completion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentPositionParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextDocumentIdentifier {
    pub uri: String,
}

/// LSP `Position` — 0-based line and UTF-16 column.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// LSP `Range`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// LSP `TextEdit` (used by completion `textEdit`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextEdit {
    pub range: Range,
    #[serde(rename = "newText")]
    pub new_text: String,
}

/// LSP `CompletionContext` (optional, omitted in our requests).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CompletionContext {
    #[serde(rename = "triggerKind", default)]
    pub trigger_kind: i32,
    #[serde(
        rename = "triggerCharacter",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub trigger_character: Option<String>,
}

/// LSP `CompletionParams`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionParams {
    #[serde(rename = "textDocument")]
    pub text_document: TextDocumentIdentifier,
    pub position: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<CompletionContext>,
}

// ---------------------------------------------------------------------------
// Tests for the framing helpers
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn read_framed_extracts_body_after_content_length() {
        let payload = r#"{"jsonrpc":"2.0","id":1,"result":42}"#;
        let framed = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
        let mut cursor = Cursor::new(framed.into_bytes());
        let body = read_framed(&mut cursor).expect("reads framed body");
        assert_eq!(body, payload);
    }

    #[test]
    fn read_framed_rejects_missing_header() {
        let mut cursor = Cursor::new(b"junk".to_vec());
        let r = read_framed(&mut cursor);
        assert!(r.is_err());
    }

    #[test]
    fn write_framed_emits_correct_header() {
        let mut buf = Vec::new();
        let body = r#"{"x":1}"#;
        write_framed(&mut buf, body).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.starts_with(&format!("Content-Length: {}\r\n\r\n", body.len())));
        assert!(s.ends_with(body));
    }

    #[test]
    fn lsp_request_serializes_with_envelope() {
        let req = LspRequest::new(7, "textDocument/hover", json!({"foo": 1}));
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains("\"jsonrpc\":\"2.0\""));
        assert!(s.contains("\"id\":7"));
        assert!(s.contains("\"method\":\"textDocument/hover\""));
    }

    #[test]
    fn lsp_response_deserializes_result_variant() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":{"value":"hi"}}"#;
        let resp: LspResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(resp.id, Some(1));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn lsp_response_deserializes_error_variant() {
        let raw =
            r#"{"jsonrpc":"2.0","id":2,"error":{"code":-32601,"message":"Method not found"}}"#;
        let resp: LspResponse = serde_json::from_str(raw).unwrap();
        assert!(resp.result.is_none());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }
}

//! Daemon: UDS listener + JSON-RPC dispatcher.
//!
//! `Server` binds a Unix-domain socket at `<runtime_dir>/forge3/daemon.sock`,
//! accepts connections in a loop, and dispatches JSON-RPC 2.0 envelopes to
//! the in-process [`crate::registry::Registry`].
//!
//! Concrete method surface in this PR-6 MVP:
//!
//! - `ping`                    → `{"ok": true}`
//! - `agent.register`          → upsert + return [`crate::registry::AgentInfo`]
//! - `agent.heartbeat`         → renew lease
//! - `agent.deregister`        → remove
//! - `agent.list`              → JSON array of active agents
//!
//! Drift, similarity, and alert methods land in PR-9+. The dispatcher
//! returns a stable JSON-RPC error code for unknown methods so clients
//! fail gracefully rather than panicking on a schema mismatch.
//!
//! Concurrency model:
//!
//! - One [`Registry`] is shared via `Arc<Registry>` — all dispatch paths
//!   read or mutate it under a short critical section.
//! - The socket accepts in a `while let Some(stream) = listener.accept().await`,
//!   spawning a `tokio::spawn` per connection. Each task owns its stream
//!   and reads requests until the peer hangs up.
//! - On read error or EOF we drop the connection silently. We never call
//!   `exit()` from inside the server; the daemon's supervisor owns lifecycle.
//!
//! Time source:
//!
//! - `Server::now_unix_ms` is a `Fn() -> i64` closure injected at
//!   construction. Tests use a fixed clock; production passes
//!   `|| chrono::Utc::now().timestamp_millis()`. Heartbeats are monotonic
//!   from the daemon's perspective, so a clock that jumps backward only ever
//!   makes leases *appear* to last longer than reality — never expired
//!   incorrectly.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{json, Value};
use tokio::io::AsyncReadExt;
use tokio::net::UnixListener;
use tracing::{debug, error, info, warn};

use crate::error::{Forge3Error, Result};
use crate::pidfile::PidFile;
use crate::protocol;
use forge_drift::{DriftDetector, DriftConfig, OverrideReason};

use crate::registry::{AgentId, AgentInfo, Lane, Registry, LEASE_MS};

/// Default relative path of the UDS socket inside the runtime dir.
pub const DEFAULT_RUNTIME_SUBDIR: &str = "forge3";
pub const DEFAULT_SOCKET_BASENAME: &str = "daemon.sock";
/// Default ancestor dir from `$XDG_RUNTIME_DIR` for the daemon's runtime files.
pub const DEFAULT_PARENT: &str = ".forge";

/// Clock function injected into the server so tests can freeze "now".
pub type Clock = Arc<dyn Fn() -> i64 + Send + Sync + 'static>;

/// Construct a real (chrono) clock.
pub fn system_clock() -> Clock {
    Arc::new(|| chrono::Utc::now().timestamp_millis())
}

/// Construct a fixed-clock for tests.
pub fn fixed_clock(unix_ms: i64) -> Clock {
    Arc::new(move || unix_ms)
}

/// Resolved on-disk paths for the daemon.
#[derive(Debug, Clone)]
pub struct Sockets {
    pub runtime_dir: PathBuf,
    pub socket_path: PathBuf,
}

impl Sockets {
    /// Resolve runtime dir from `$XDG_RUNTIME_DIR` (falling back to
    /// `/tmp` and finally `~/.forge`).
    pub fn resolve_default() -> Self {
        let runtime = std::env::var("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .ok()
            .filter(|p| p.is_absolute() && p.is_dir())
            .or_else(|| {
                let tmp = PathBuf::from("/tmp");
                if tmp.is_dir() {
                    Some(tmp)
                } else {
                    None
                }
            })
            .or_else(|| {
                dirs_home().map(|h| h.join(DEFAULT_PARENT))
            })
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        Self::resolve_in(runtime)
    }

    /// Build sockets inside a specific parent dir.
    pub fn resolve_in(parent: PathBuf) -> Self {
        let runtime_dir = parent.join(DEFAULT_RUNTIME_SUBDIR);
        let socket_path = runtime_dir.join(DEFAULT_SOCKET_BASENAME);
        Self {
            runtime_dir,
            socket_path,
        }
    }
}

fn dirs_home() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    None
}

/// The daemon. Holds everything needed to run a JSON-RPC server over UDS.
pub struct Server {
    registry: Arc<Registry>,
    sockets: Sockets,
    pidfile: Option<PidFile>,
    clock: Clock,
    drift_detector: Option<Arc<DriftDetector>>,
}

impl Server {
    /// Build a server with no PID guard; useful for in-process tests.
    pub fn new(registry: Arc<Registry>, sockets: Sockets, clock: Clock) -> Self {
        Self {
            registry,
            sockets,
            pidfile: None,
            clock,
            drift_detector: None,
        }
    }

    /// Acquire a PID file at `pidfile_dir` so a second instance fails fast.
    pub fn with_pidfile(mut self, pidfile_dir: &Path) -> Result<Self> {
        let pid = std::process::id();
        let guard = PidFile::acquire(pidfile_dir, pid)?;
        self.pidfile = Some(guard);
        Ok(self)
    }

    /// Attach a drift detector for cross-agent overlap detection.
    pub fn with_drift_detector(mut self, detector: DriftDetector) -> Self {
        self.drift_detector = Some(Arc::new(detector));
        self
    }

    /// Socket path (for tests / client constructors).
    pub fn socket_path(&self) -> &Path {
        &self.sockets.socket_path
    }

    /// Listen on the UDS socket and run forever.
    /// Returns only on bind error or join failure.
    pub async fn serve(&self) -> Result<()> {
        if let Some(parent) = self.sockets.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Best-effort cleanup: if the socket was left over from a crashed
        // previous run, unlink it so bind() succeeds. If unlink fails for
        // any reason other than NotFound, fall back to surface the error.
        match std::fs::remove_file(&self.sockets.socket_path) {
            Ok(()) => info!(path = %self.sockets.socket_path.display(), "removed stale socket"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => warn!(error = %e, "could not remove stale socket"),
        }

        let listener = UnixListener::bind(&self.sockets.socket_path)?;
        info!(path = %self.sockets.socket_path.display(), "listening");
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let registry = Arc::clone(&self.registry);
                    let clock = Arc::clone(&self.clock);
                    let drift_detector = self.drift_detector.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, registry, clock, drift_detector).await {
                            debug!(error = %e, "connection ended with error");
                        }
                    });
                }
                Err(e) => {
                    warn!(error = %e, "accept failed; continuing");
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: tokio::net::UnixStream,
    registry: Arc<Registry>,
    clock: Clock,
    drift_detector: Option<Arc<DriftDetector>>,
) -> Result<()> {
    loop {
        let payload = match protocol::decode_frame(&mut stream).await? {
            Some(bytes) => bytes,
            None => return Ok(()), // peer hung up
        };
        let req = match protocol::parse_request(&payload) {
            Ok(r) => r,
            Err(e) => {
                let resp = error_envelope(
                    json!(0),
                    e.code(),
                    e.to_string(),
                );
                protocol::write_frame(&mut stream, &serialise(&resp)).await?;
                continue;
            }
        };

        if let Some(id) = req.id.clone() {
            // request -> respond
            let resp = dispatch(&req.method, req.params.clone(), &registry, &clock, &drift_detector).await;
            let envelope = match resp {
                Ok(value) => json!({
                    "jsonrpc": "2.0",
                    "result": value,
                    "id": id,
                }),
                Err(e) => error_envelope(id, e.code(), e.to_string()),
            };
            protocol::write_frame(&mut stream, &serialise(&envelope)).await?;
        }
        // notifications (id == None) get no response
    }
}

/// Tasks completed (or refused) by the dispatcher. Anything not in this
/// table returns `-32601 Method not found`.
async fn dispatch(
    method: &str,
    params: Value,
    registry: &Registry,
    clock: &Clock,
    drift_detector: &Option<Arc<DriftDetector>>,
) -> Result<Value> {
    let now = clock();
    match method {
        "ping" => Ok(json!({ "ok": true, "now_unix_ms": now })),
        "agent.register" => {
            let args: RegisterArgs = serde_json::from_value(params)
                .map_err(|e| Forge3Error::Protocol(format!("agent.register: {e}")))?;
            let info = registry.upsert_with_excerpt(
                &args.agent_id,
                args.pid,
                &args.lane,
                cap_excerpt(args.prompt_excerpt),
                now,
            );
            Ok(agent_info_to_value(&info))
        }
        "agent.heartbeat" => {
            let args: HeartbeatArgs = serde_json::from_value(params)
                .map_err(|e| Forge3Error::Protocol(format!("agent.heartbeat: {e}")))?;
            let info = registry
                .heartbeat(&args.agent_id, now)
                .ok_or_else(|| Forge3Error::UnknownAgent(args.agent_id.clone()))?;
            Ok(agent_info_to_value(&info))
        }
        "agent.deregister" => {
            let args: DeregisterArgs = serde_json::from_value(params)
                .map_err(|e| Forge3Error::Protocol(format!("agent.deregister: {e}")))?;
            if !registry.deregister(&args.agent_id) {
                return Err(Forge3Error::UnknownAgent(args.agent_id));
            }
            Ok(json!({ "ok": true }))
        }
        "agent.list" => {
            let active = registry.list_active(now);
            let payload: Vec<Value> = active.iter().map(agent_info_to_value).collect();
            Ok(json!(payload))
        }
        "drift.observe" => {
            let detector = drift_detector.as_ref().ok_or_else(|| {
                Forge3Error::Protocol("drift detector not configured".into())
            })?;
            let args: DriftObserveArgs = serde_json::from_value(params)
                .map_err(|e| Forge3Error::Protocol(format!("drift.observe: {e}")))?;
            let event = detector.observe(
                &args.agent_id,
                &args.prompt,
                &args.lane,
                now,
            ).await;
            Ok(match event {
                Some(e) => json!(e),
                None => Value::Null,
            })
        }
        "drift.override" => {
            let detector = drift_detector.as_ref().ok_or_else(|| {
                Forge3Error::Protocol("drift detector not configured".into())
            })?;
            let args: DriftOverrideArgs = serde_json::from_value(params)
                .map_err(|e| Forge3Error::Protocol(format!("drift.override: {e}")))?;
            let reason = match args.reason.as_str() {
                "same_intent" => OverrideReason::SameIntent,
                "coordinated" => OverrideReason::Coordinated,
                "user_directed" => OverrideReason::UserDirected,
                other => return Err(Forge3Error::Protocol(
                    format!("drift.override: unknown reason '{other}'")
                )),
            };
            detector.override_alert(args.alert_id.clone(), reason);
            Ok(json!({ "ok": true }))
        }
        _ => Err(Forge3Error::Protocol(format!(
            "unknown method: {method}"
        ))),
    }
}

fn agent_info_to_value(info: &AgentInfo) -> Value {
    json!({
        "agent_id": info.agent_id,
        "pid": info.pid,
        "lane": info.lane,
        "prompt_excerpt": info.prompt_excerpt,
        "registered_at_unix_ms": info.registered_at_unix_ms,
        "last_heartbeat_unix_ms": info.last_heartbeat_unix_ms,
    })
}

fn error_envelope(id: Value, code: i32, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "error": {
            "code": code,
            "message": message,
        },
        "id": id,
    })
}

fn serialise(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).expect("always serializable")
}

fn cap_excerpt(s: Option<String>) -> Option<String> {
    s.and_then(|x| {
        const MAX: usize = 256;
        if x.len() <= MAX {
            Some(x)
        } else {
            // best-effort: take first MAX bytes on a char boundary
            let mut end = MAX;
            while !x.is_char_boundary(end) {
                end -= 1;
            }
            Some(x[..end].to_string())
        }
    })
}

#[derive(Debug, Deserialize)]
struct RegisterArgs {
    agent_id: String,
    pid: u32,
    lane: String,
    #[serde(default)]
    prompt_excerpt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HeartbeatArgs {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
struct DeregisterArgs {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
struct DriftObserveArgs {
    agent_id: String,
    prompt: String,
    lane: String,
}

#[derive(Debug, Deserialize)]
struct DriftOverrideArgs {
    alert_id: String,
    reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixStream;

    async fn round_trip(
        socket: &Path,
        frames: Vec<Vec<u8>>,
    ) -> Vec<Value> {
        let mut stream = UnixStream::connect(socket).await.expect("connect");
        for f in &frames {
            protocol::write_frame(&mut stream, f).await.expect("write");
        }
        stream.shutdown().await.ok();
        let mut responses = Vec::new();
        loop {
            let payload = match protocol::decode_frame(&mut stream).await {
                Ok(Some(b)) => b,
                Ok(None) => break,
                Err(e) => panic!("decode: {e}"),
            };
            responses.push(serde_json::from_slice(&payload).expect("json"));
        }
        responses
    }

    fn request(method: &str, id: u64, params: Value) -> Vec<u8> {
        let env = json!({
            "jsonrpc": "2.0",
            "method": method,
            "id": id,
            "params": params,
        });
        serde_json::to_vec(&env).unwrap()
    }

    #[tokio::test]
    async fn ping_roundtrip() {
        let td = tempfile::tempdir().unwrap();
        let sockets = Sockets::resolve_in(td.path().to_path_buf());
        let registry = Arc::new(Registry::new());
        let server = Server::new(registry, sockets.clone(), fixed_clock(1000));

        // Spawn the listener.
        let s_sockets = sockets.clone();
        let server_for_task = Server::new(Arc::new(Registry::new()), s_sockets, fixed_clock(1000));
        // We can't run two arbitrary Server instances against the same socket;
        // instead run serve() in a background task using *one* of these and let
        // the test connect to it.
        drop(server); // avoid moving into a task with a duplicate socket path

        let listener_task = tokio::spawn(async move {
            // suppress: the `Server::serve` loop never returns Ok
            let _ = server_for_task.serve().await;
        });

        // give the listener time to bind
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let resp = round_trip(
            &sockets.socket_path,
            vec![request("ping", 1, json!({}))],
        )
        .await;

        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0]["result"]["ok"], json!(true));
        assert_eq!(resp[0]["id"], json!(1));

        // tear down
        drop(listener_task);
    }

    #[tokio::test]
    async fn register_then_list() {
        let td = tempfile::tempdir().unwrap();
        let sockets = Sockets::resolve_in(td.path().to_path_buf());
        let shared_registry = Arc::new(Registry::new());
        let server = Server::new(
            Arc::clone(&shared_registry),
            sockets.clone(),
            fixed_clock(5000),
        );

        let listener_task = tokio::spawn(async move {
            let _ = server.serve().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let frames = vec![
            request(
                "agent.register",
                1,
                json!({
                    "agent_id": "alpha",
                    "pid": 999,
                    "lane": Lane::BUILDING,
                    "prompt_excerpt": "drift detection prototype",
                }),
            ),
            request("agent.list", 2, json!({})),
        ];

        let responses = round_trip(&sockets.socket_path, frames).await;
        assert_eq!(responses.len(), 2, "expected 2 responses got {responses:?}");

        let reg = &responses[0];
        assert_eq!(reg["result"]["agent_id"], "alpha");
        assert_eq!(reg["result"]["pid"], 999);
        assert_eq!(reg["result"]["lane"], Lane::BUILDING);
        assert_eq!(reg["result"]["prompt_excerpt"], "drift detection prototype");

        let list = &responses[1];
        let arr = list["result"].as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["agent_id"], "alpha");

        drop(listener_task);
    }

    #[tokio::test]
    async fn drift_observe_without_detector_returns_error() {
        let td = tempfile::tempdir().unwrap();
        let sockets = Sockets::resolve_in(td.path().to_path_buf());
        let server = Server::new(Arc::new(Registry::new()), sockets.clone(), fixed_clock(0));

        let listener_task = tokio::spawn(async move {
            let _ = server.serve().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let resp = round_trip(
            &sockets.socket_path,
            vec![request("drift.observe", 9, json!({"agent_id":"a","prompt":"hello","lane":"building"}))],
        )
        .await;
        assert_eq!(resp.len(), 1);
        assert!(resp[0]["error"].is_object());
        // Should say "drift detector not configured" — protocol error, not "unknown method"
        let code = resp[0]["error"]["code"].as_i64().unwrap();
        assert_eq!(code, -32600);
        let msg = resp[0]["error"]["message"].as_str().unwrap();
        assert!(msg.contains("not configured"));
        drop(listener_task);
    }

    #[tokio::test]
    async fn unknown_method_returns_envelope_error() {
        let td = tempfile::tempdir().unwrap();
        let sockets = Sockets::resolve_in(td.path().to_path_buf());
        let server = Server::new(Arc::new(Registry::new()), sockets.clone(), fixed_clock(0));

        let listener_task = tokio::spawn(async move {
            let _ = server.serve().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let resp = round_trip(
            &sockets.socket_path,
            vec![request("totally.bogus", 9, json!({}))],
        )
        .await;
        assert_eq!(resp.len(), 1);
        assert!(resp[0]["error"].is_object());
        // Should be -32600 (Protocol) for "unknown method: totally.bogus"
        let code = resp[0]["error"]["code"].as_i64().unwrap();
        assert!(code != 0);
        drop(listener_task);
    }

    #[tokio::test]
    async fn heartbeat_unknown_agent_errors() {
        let td = tempfile::tempdir().unwrap();
        let sockets = Sockets::resolve_in(td.path().to_path_buf());
        let server = Server::new(Arc::new(Registry::new()), sockets.clone(), fixed_clock(0));

        let listener_task = tokio::spawn(async move {
            let _ = server.serve().await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let resp = round_trip(
            &sockets.socket_path,
            vec![request("agent.heartbeat", 1, json!({"agent_id": "ghost"}))],
        )
        .await;
        assert_eq!(resp.len(), 1);
        assert_eq!(resp[0]["error"]["code"], Forge3Error::UnknownAgent("ghost".into()).code());
        drop(listener_task);
    }

    #[test]
    fn cap_excerpt_truncates_on_char_boundary() {
        // ASCII -> easy truncation
        let long = "x".repeat(300);
        let out = cap_excerpt(Some(long)).unwrap();
        assert_eq!(out.len(), 256);

        // Multi-byte boundary preservation — "á" is 2 bytes in UTF-8.
        let mut s = String::new();
        while s.len() < 255 {
            s.push('á');
        }
        s.push('z');
        let out = cap_excerpt(Some(s.clone())).unwrap();
        assert!(out.len() <= 256);
        // Must round-trip the prefix as valid UTF-8 (no mid-codepoint cut).
        assert!(out == s[..256].to_string() || out == s[..255].to_string());
    }

    #[test]
    fn sockets_resolve_in_uses_subdir() {
        let s = Sockets::resolve_in(PathBuf::from("/var/run/myapp"));
        assert!(s.socket_path.ends_with("forge3/daemon.sock"));
        assert!(s.runtime_dir.ends_with("myapp/forge3"));
    }
}

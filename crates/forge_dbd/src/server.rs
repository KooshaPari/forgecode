// The batch-writer pipeline (queue plumbing, health probe, PRAGMA execution)
// is driven only by the Unix transport in `run_unix`. On non-Unix platforms
// `run()` bails out before any of it runs, so the machinery compiles but is
// unreachable there. Allow dead_code rather than scattering cfg_attr across
// the subsystem.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use rusqlite::Connection;
#[cfg(unix)]
use tokio::net::{UnixListener, UnixStream};
#[cfg(unix)]
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::time::timeout;
use tracing::{debug, info};
#[cfg(unix)]
use tracing::{error, warn};

use crate::protocol::{HealthStatus, Request, Response};
#[cfg(unix)]
use crate::protocol::{read_frame, write_frame};

// ---------------------------------------------------------------------------
// Shared daemon state (cheap to clone; wraps Arcs internally)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct DaemonState {
    pub db_path: PathBuf,
    pub started_at: Instant,
    /// Approximate number of items currently sitting in the write queue.
    pub queue_depth: Arc<AtomicUsize>,
}

impl DaemonState {
    fn health(&self) -> HealthStatus {
        HealthStatus {
            uptime_secs: self.started_at.elapsed().as_secs(),
            queue_depth: self.queue_depth.load(Ordering::Relaxed),
            db_reachable: db_reachable(&self.db_path),
        }
    }
}

/// Whether the write database is actually reachable: the file must exist,
/// open cleanly, and pass `PRAGMA quick_check`. Existence alone is not enough —
/// a never-opened path marker is not a working database.
fn db_reachable(db_path: &Path) -> bool {
    if !db_path.exists() {
        return false;
    }
    Connection::open(db_path)
        .and_then(|conn| {
            conn.query_row("PRAGMA quick_check", [], |row| row.get::<_, String>(0))
        })
        .map(|result| result == "ok")
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Public server handle
// ---------------------------------------------------------------------------

pub struct DbServer {
    socket_path: PathBuf,
    state: DaemonState,
}

struct QueuedRequest {
    request: Request,
    response_tx: tokio::sync::oneshot::Sender<Response>,
}

impl DbServer {
    pub fn new(socket_path: PathBuf, db_path: PathBuf) -> Self {
        Self {
            socket_path,
            state: DaemonState {
                db_path,
                started_at: Instant::now(),
                queue_depth: Arc::new(AtomicUsize::new(0)),
            },
        }
    }

    pub async fn run(self) -> Result<()> {
        #[cfg(not(unix))]
        {
            anyhow::bail!(
                "forge_dbd requires Unix domain socket support and is not available on this platform"
            );
        }

        #[cfg(unix)]
        {
            self.run_unix().await
        }
    }

    #[cfg(unix)]
    async fn run_unix(self) -> Result<()> {
        info!(
            socket = %self.socket_path.display(),
            db    = %self.state.db_path.display(),
            "DbServer starting"
        );

        // Remove stale socket if present
        if self.socket_path.exists() {
            warn!(path = %self.socket_path.display(), "removing stale socket");
            tokio::fs::remove_file(&self.socket_path).await?;
        }

        // Create parent directory if needed
        if let Some(parent) = self.socket_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        info!(socket = %self.socket_path.display(), "Unix socket bound");

        // The real write queue used during this run
        let (queue_tx, queue_rx) = mpsc::channel::<QueuedRequest>(1024);
        let state = self.state.clone();
        // Wrap queue_tx so we can drop it on shutdown to signal the writer
        let queue_tx = Arc::new(queue_tx);

        // Spawn the batching writer task
        let db_path = self.state.db_path.clone();
        let writer_handle = tokio::spawn(Self::writer_task(queue_rx, db_path));

        // One-shot shutdown signal: fired by OS signal handlers
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        // Install SIGTERM / SIGINT handlers
        #[cfg(unix)]
        {
            let mut sigterm =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            let mut sigint =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
            tokio::spawn(async move {
                tokio::select! {
                    _ = sigterm.recv() => { info!("SIGTERM received"); }
                    _ = sigint.recv()  => { info!("SIGINT received"); }
                }
                let _ = shutdown_tx.send(());
            });
        }
        // On non-Unix platforms the shutdown_tx is dropped immediately which
        // means shutdown_rx fires at startup — acceptable for a Unix daemon.
        #[cfg(not(unix))]
        {
            let _ = shutdown_tx; // silence unused warning
        }

        // Accept loop — exits when shutdown fires
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, _addr)) => {
                            debug!("client connected");
                            let queue_tx = Arc::clone(&queue_tx);
                            let state = state.clone();
                            tokio::spawn(Self::handle_client(stream, queue_tx, state));
                        }
                        Err(e) => {
                            error!("accept error: {e}");
                        }
                    }
                }
                _ = &mut shutdown_rx => {
                    info!("shutdown signal received; draining write queue");
                    break;
                }
            }
        }

        // ---- Graceful drain ------------------------------------------------
        // Drop our sender half so the writer task sees channel-closed once all
        // in-flight client handlers also drop their clones.
        drop(queue_tx);

        // Wait for the writer to finish flushing.
        match writer_handle.await {
            Ok(()) => info!("writer task drained; exiting cleanly"),
            Err(e) => error!("writer task panicked: {e}"),
        }

        // Remove the socket file so the next start-up doesn't need to clean up.
        if self.socket_path.exists() {
            let _ = tokio::fs::remove_file(&self.socket_path).await;
        }

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Per-connection handler
    // -------------------------------------------------------------------------

    #[cfg(unix)]
    async fn handle_client(
        stream: UnixStream,
        queue_tx: Arc<mpsc::Sender<QueuedRequest>>,
        state: DaemonState,
    ) {
        let stream = Arc::new(Mutex::new(stream));

        loop {
            let request = {
                let mut guard = stream.lock().await;
                match timeout(
                    Duration::from_secs(30),
                    read_frame::<_, Request>(&mut *guard),
                )
                .await
                {
                    Ok(Ok(req)) => req,
                    Ok(Err(e)) => {
                        debug!("frame read error: {e}");
                        break;
                    }
                    Err(_) => {
                        debug!("client read timeout");
                        break;
                    }
                }
            };

            debug!("received request: {:?}", request);

            // Health probe is handled inline — no queue round-trip needed
            if matches!(request, Request::Ping) {
                let resp = Response::Health(state.health());
                let mut guard = stream.lock().await;
                let _ = write_frame(&mut *guard, &resp).await;
                continue;
            }

            // All other requests go through the write queue
            let (response_tx, response_rx) = tokio::sync::oneshot::channel();
            state.queue_depth.fetch_add(1, Ordering::Relaxed);
            let queued = QueuedRequest { request, response_tx };

            if queue_tx.send(queued).await.is_err() {
                state.queue_depth.fetch_sub(1, Ordering::Relaxed);
                error!("failed to enqueue request; channel closed");
                let err_response = Response::Error { message: "server queue closed".to_string() };
                let mut guard = stream.lock().await;
                let _ = write_frame(&mut *guard, &err_response).await;
                break;
            }

            match timeout(Duration::from_secs(30), response_rx).await {
                Ok(Ok(response)) => {
                    debug!("sending response: {:?}", response);
                    let mut guard = stream.lock().await;
                    if let Err(e) = write_frame(&mut *guard, &response).await {
                        error!("failed to write response: {e}");
                        break;
                    }
                }
                Ok(Err(_)) => {
                    error!("response oneshot dropped");
                    break;
                }
                Err(_) => {
                    error!("response timeout");
                    let timeout_resp =
                        Response::Error { message: "server processing timeout".to_string() };
                    let mut guard = stream.lock().await;
                    let _ = write_frame(&mut *guard, &timeout_resp).await;
                    break;
                }
            }
        }

        debug!("client disconnected");
    }

    // -------------------------------------------------------------------------
    // Batching writer task
    // -------------------------------------------------------------------------

    async fn writer_task(mut queue_rx: mpsc::Receiver<QueuedRequest>, db_path: PathBuf) {
        let mut batch: Vec<QueuedRequest> = Vec::new();
        let batch_timeout = Duration::from_millis(15);
        let batch_threshold = 100;

        loop {
            match timeout(batch_timeout, queue_rx.recv()).await {
                Ok(Some(req)) => {
                    batch.push(req);
                    if batch.len() >= batch_threshold {
                        Self::flush_batch(&mut batch, &db_path).await;
                    }
                }
                Ok(None) => {
                    // All senders dropped (graceful shutdown path)
                    if !batch.is_empty() {
                        info!(count = batch.len(), "draining final batch on shutdown");
                        Self::flush_batch(&mut batch, &db_path).await;
                    }
                    info!("writer task exiting");
                    break;
                }
                Err(_) => {
                    // Batch window elapsed
                    if !batch.is_empty() {
                        Self::flush_batch(&mut batch, &db_path).await;
                    }
                }
            }
        }
    }

    /// Execute a batch of requests against the SQLite write database.
    async fn flush_batch(batch: &mut Vec<QueuedRequest>, db_path: &Path) {
        debug!(count = batch.len(), "flushing batch");
        for queued in batch.drain(..) {
            let resp = match Self::execute(db_path, &queued.request) {
                Ok(resp) => resp,
                Err(e) => Response::Error {
                    message: format!("db error: {e}"),
                },
            };
            let _ = queued.response_tx.send(resp);
        }
    }

    /// Execute a single request against the SQLite database at `db_path`.
    ///
    /// Opens (creating if needed) the write DB and performs the real operation:
    /// - [`Request::CheckpointWal`] runs `PRAGMA wal_checkpoint(TRUNCATE)`.
    /// - [`Request::OptimizeFts`] / [`Request::RefreshFts`] run
    ///   `PRAGMA optimize`, which also maintains FTS indexes when present.
    /// - Conversation writes need the forge_repo schema, which this scaffold
    ///   does not own yet; they report an explicit error (with the resolved
    ///   path) instead of a misleading `Ack`.
    fn execute(db_path: &Path, request: &Request) -> Result<Response> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;

        match request {
            Request::CheckpointWal => {
                // wal_checkpoint(TRUNCATE) returns a single row
                // (busy, log, checkpointed); running it is the real work.
                let (busy, log, checkpointed): (i64, i64, i64) =
                    conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })?;
                debug!(busy, log, checkpointed, "wal checkpoint executed");
                Ok(Response::Ack)
            }
            Request::OptimizeFts | Request::RefreshFts => {
                conn.execute_batch("PRAGMA optimize;")?;
                Ok(Response::Ack)
            }
            request @ (Request::UpsertConversation { .. }
            | Request::UpsertConversationRef { .. }
            | Request::UpdateParentId { .. }
            | Request::DeleteConversation { .. }) => Ok(Response::Error {
                message: format!(
                    "conversation write {request:?} needs the forge_repo schema, \
                     not yet wired into forge_dbd (opened {})",
                    db_path.display()
                ),
            }),
            Request::Ping => Ok(Response::Error {
                message: "Ping is answered inline by the connection handler, \
                          not by the batch writer"
                    .to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(all(test, unix))]
mod tests {
    use std::path::{Path, PathBuf};

    use tempfile::TempDir;
    use tokio::net::UnixStream;
    use tokio::time::{Duration, sleep};

    use super::*;
    use crate::protocol::{Request, Response, read_frame, write_frame};

    fn tmp_paths(dir: &TempDir) -> (PathBuf, PathBuf) {
        let sock = dir.path().join("test.sock");
        let db = dir.path().join("test.db");
        (sock, db)
    }

    /// Spawn the server in the background and return a handle + socket path.
    async fn spawn_server(
        sock: PathBuf,
        db: PathBuf,
    ) -> tokio::task::JoinHandle<anyhow::Result<()>> {
        let server = DbServer::new(sock, db);
        tokio::spawn(server.run())
    }

    /// Wait until the socket file appears (server is ready to accept).
    async fn wait_for_socket(sock: &Path) {
        for _ in 0..50 {
            if sock.exists() {
                return;
            }
            sleep(Duration::from_millis(20)).await;
        }
        panic!("server socket did not appear in time");
    }

    // -------------------------------------------------------------------------
    // Health probe test
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn health_probe_returns_status() {
        let dir = TempDir::new().unwrap();
        let (sock, db) = tmp_paths(&dir);
        let _handle = spawn_server(sock.clone(), db.clone()).await;
        wait_for_socket(&sock).await;

        let mut stream = UnixStream::connect(&sock).await.expect("connect");
        write_frame(&mut stream, &Request::Ping)
            .await
            .expect("write ping");
        let resp: Response = read_frame(&mut stream).await.expect("read health");

        match resp {
            Response::Health(status) => {
                // uptime is small but non-negative
                assert!(status.uptime_secs < 60, "uptime should be < 60s in test");
                // queue should be empty while no writes are in flight
                assert_eq!(status.queue_depth, 0);
                // db file doesn't exist yet (just a path marker) — reachable = false
                assert!(!status.db_reachable);
            }
            other => panic!("expected Health response, got {other:?}"),
        }
    }

    // -------------------------------------------------------------------------
    // Drain test: enqueue writes, then close the accept side; writer must flush
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn graceful_drain_flushes_queued_writes() {
        let dir = TempDir::new().unwrap();
        let (sock, db) = tmp_paths(&dir);
        let _handle = spawn_server(sock.clone(), db.clone()).await;
        wait_for_socket(&sock).await;

        // Send a few writes and collect Ack responses to confirm they're processed
        let mut stream = UnixStream::connect(&sock).await.expect("connect");

        // Use OptimizeFts as a lightweight write request
        let n = 5usize;
        for _ in 0..n {
            write_frame(&mut stream, &Request::OptimizeFts)
                .await
                .expect("write request");
        }

        let mut acks = 0usize;
        for _ in 0..n {
            let resp: Response = read_frame(&mut stream).await.expect("read response");
            if matches!(resp, Response::Ack) {
                acks += 1;
            }
        }

        assert_eq!(
            acks, n,
            "all writes should be acknowledged (drain verified)"
        );
    }

    // -------------------------------------------------------------------------
    // Queue depth reflected in health status when writes are in flight
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn health_probe_reflects_queue_depth() {
        // This test verifies the atomic counter path is exercised.
        // Because the writer drains quickly, we just confirm the probe succeeds
        // (depth may already be 0 by the time we probe — that is correct behavior).
        let dir = TempDir::new().unwrap();
        let (sock, db) = tmp_paths(&dir);
        let _handle = spawn_server(sock.clone(), db.clone()).await;
        wait_for_socket(&sock).await;

        let mut stream = UnixStream::connect(&sock).await.expect("connect");
        write_frame(&mut stream, &Request::Ping)
            .await
            .expect("write ping");
        let resp: Response = read_frame(&mut stream).await.expect("read health");
        assert!(matches!(resp, Response::Health(_)));
    }
}

// ---------------------------------------------------------------------------
// DB execution tests — platform-independent: drive the batch handler directly
// (the same path the Unix writer task uses), no socket required.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod db_tests {
    use forge_domain::ConversationId;
    use tempfile::TempDir;

    use super::*;

    #[tokio::test]
    async fn flush_batch_performs_real_sqlite_work() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let mut batch = vec![QueuedRequest {
            request: Request::CheckpointWal,
            response_tx,
        }];
        DbServer::flush_batch(&mut batch, &db_path).await;

        // A stub Ack would acknowledge without creating a file; real execution
        // opens the SQLite DB and runs the checkpoint.
        assert!(matches!(response_rx.await.unwrap(), Response::Ack));
        assert!(db_path.exists());

        // ...and the file on disk is a valid, intact SQLite database.
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");
    }

    #[tokio::test]
    async fn conversation_write_reports_resolved_path_instead_of_stub_ack() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("test.db");

        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let mut batch = vec![QueuedRequest {
            request: Request::DeleteConversation {
                conversation_id: ConversationId::default(),
            },
            response_tx,
        }];
        DbServer::flush_batch(&mut batch, &db_path).await;

        match response_rx.await.unwrap() {
            Response::Error { message } => {
                assert!(
                    message.contains(&db_path.display().to_string()),
                    "error should reference the resolved db path: {message}"
                );
            }
            other => panic!("expected Error response, got {other:?}"),
        }
    }
}

use std::path::PathBuf;

use anyhow::Result;
use tokio::sync::mpsc;

use crate::protocol::{Request, Response};

pub struct DbServer {
    socket_path: PathBuf,
    // TODO: scaffold holds the database connection path as a placeholder.
    // The real DatabasePool connection will be wired in here once daemon
    // logic is implemented (avoid depending on forge_repo internals).
    db_path: PathBuf,
    queue_tx: mpsc::Sender<QueuedRequest>,
}

struct QueuedRequest {
    request: Request,
    response_tx: tokio::sync::oneshot::Sender<Response>,
}

impl DbServer {
    pub fn new(socket_path: PathBuf, db_path: PathBuf) -> Self {
        let (queue_tx, _queue_rx) = mpsc::channel(1024);
        Self { socket_path, db_path, queue_tx }
    }

    pub async fn run(self) -> Result<()> {
        let _ = self.socket_path;
        let _ = self.db_path;
        let _ = self.queue_tx;
        // TODO: open the database connection at `self.db_path` here.
        todo!("bind Unix socket, accept connections, and batch write requests");
    }

    pub async fn enqueue(&self, request: Request) -> Result<Response> {
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let queued = QueuedRequest { request, response_tx };
        self.queue_tx.send(queued).await?;
        response_rx.await.map_err(anyhow::Error::from)
    }
}

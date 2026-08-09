mod client;
mod protocol;
mod server;

use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

fn socket_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".forge").join(".forge.db.sock")
}

/// Resolves the daemon's write database path.
///
/// Mirrors [`forge_domain::Environment::write_database_path`]:
/// 1. `FORGE_WRITE_DB_PATH` environment variable, if set.
/// 2. Otherwise the split-DB default `~/.forge/.forge.writes.db`, keeping the
///    legacy `~/.forge/.forge.db` untouched for the read-side UNION.
fn db_path() -> PathBuf {
    if let Ok(path) = std::env::var("FORGE_WRITE_DB_PATH") {
        return PathBuf::from(path);
    }
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".forge").join(".forge.writes.db")
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    let socket_path = socket_path();
    let db_path = db_path();
    info!(socket = %socket_path.display(), "starting forge-dbd");

    let server = server::DbServer::new(socket_path, db_path);
    server.run().await
}

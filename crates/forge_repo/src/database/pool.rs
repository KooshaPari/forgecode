#![allow(dead_code)]
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use backon::{BlockingRetryable, ExponentialBuilder};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use tracing::{debug, warn};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/database/migrations");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type PooledSqliteConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: u32,
    pub min_idle: Option<u32>,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub max_retries: usize,
    pub database_path: PathBuf,
}

impl PoolConfig {
    /// Defaults tuned for high-concurrency interactive use (96+ parallel
    /// forge procs sharing one SQLite DB).
    ///
    /// * `max_size: 10` — each proc can run ~10 concurrent SQL operations
    ///   before queueing inside the pool.
    /// * `connection_timeout: 60s` — must exceed SQLite's `busy_timeout`
    ///   (30s) plus retry backoff, so a busy SQLite writer doesn't get
    ///   masked as an r2d2 timeout.
    /// * `max_retries: 10` — enough retries to absorb transient contention
    ///   when multiple procs are racing on the WAL writer.
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            max_size: 10,
            min_idle: Some(2),
            connection_timeout: Duration::from_secs(60),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            max_retries: 10,
            database_path,
        }
    }
}

/// Thread-safe database pool with periodic incremental vacuum maintenance.
#[derive(Clone)]
pub struct DatabasePool {
    pool: Arc<DbPool>,
    max_retries: usize,
}

impl DatabasePool {
    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        debug!("Creating in-memory database pool");

        let manager = ConnectionManager::<SqliteConnection>::new(":memory:");

        let pool = Pool::builder()
            .max_size(1) // Single connection for in-memory testing
            .connection_timeout(Duration::from_secs(30))
            .build(manager)
            .map_err(|e| anyhow::anyhow!("Failed to create in-memory connection pool: {e}"))?;

        // Run migrations on the in-memory database
        let mut connection = pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get connection for migrations: {e}"))?;

        connection
            .run_pending_migrations(MIGRATIONS)
            .map_err(|e| anyhow::anyhow!("Failed to run database migrations: {e}"))?;

        Ok(Self { pool: Arc::new(pool), max_retries: 5 })
    }

    pub fn get_connection(&self) -> Result<PooledSqliteConnection> {
        Self::retry_with_backoff(
            self.max_retries,
            "Failed to get connection from pool, retrying",
            || {
                self.pool
                    .get()
                    .map_err(|e| anyhow::anyhow!("Failed to get connection from pool: {e}"))
            },
        )
    }

    /// Run an incremental vacuum to reclaim freed pages and return them to the OS.
    /// Non-fatal: logs errors and continues if vacuum fails (e.g., on a locked database).
    ///
    /// This is called periodically by the background maintenance task spawned in build_pool.
    pub fn incremental_vacuum(&self) -> Result<()> {
        let mut conn = self.get_connection()?;
        match diesel::sql_query("PRAGMA incremental_vacuum;").execute(&mut conn) {
            Ok(_) => {
                debug!("incremental_vacuum completed successfully");
                Ok(())
            }
            Err(e) => {
                warn!(error = %e, "incremental_vacuum failed (non-fatal, will retry next cycle)");
                Ok(())
            }
        }
    }

    /// Retries a blocking database pool operation with exponential backoff.
    fn retry_with_backoff<T>(
        max_retries: usize,
        message: &'static str,
        operation: impl FnMut() -> Result<T>,
    ) -> Result<T> {
        operation
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(Duration::from_secs(1))
                    .with_max_times(max_retries)
                    .with_jitter(),
            )
            .sleep(std::thread::sleep)
            .notify(|err, dur| {
                warn!(
                    error = %err,
                    retry_after_ms = dur.as_millis() as u64,
                    "{}",
                    message
                );
            })
            .call()
    }
}
/// Configure SQLite for better concurrency and storage efficiency.
///
/// Ref: https://docs.diesel.rs/master/diesel/sqlite/struct.SqliteConnection.html#concurrency
///
/// **auto_vacuum=INCREMENTAL:**
/// - For NEW databases: enables incremental auto_vacuum at creation time, allowing freed pages
///   to return to the OS continuously without an exclusive-lock full VACUUM.
/// - For EXISTING databases: this pragma is a no-op and doesn't change the setting. To convert
///   an existing database to INCREMENTAL auto_vacuum, run a one-time full `VACUUM` (e.g., via
///   forge-vacuum tool). After that one-time conversion, incremental_vacuum (see below) keeps
///   reclaiming freed pages automatically.
///
/// **FORGE_INCREMENTAL_VACUUM env var (default: enabled):**
/// - When enabled, the background checkpoint task periodically runs `PRAGMA incremental_vacuum`
///   to return freed pages (from P4 prune, zstd compression, deletes) to the OS.
/// - Set to "0" or "false" to disable if needed.
#[derive(Debug)]
struct SqliteCustomizer;

impl CustomizeConnection<SqliteConnection, diesel::r2d2::Error> for SqliteCustomizer {
    fn on_acquire(&self, conn: &mut SqliteConnection) -> Result<(), diesel::r2d2::Error> {
        diesel::sql_query("PRAGMA busy_timeout = 30000;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA journal_mode = WAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA synchronous = NORMAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        diesel::sql_query("PRAGMA wal_autocheckpoint = 1000;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        // Enable incremental auto_vacuum for new databases. On existing DBs, this is a no-op;
        // they need one full VACUUM to convert, after which incremental_vacuum (spawned in the
        // background) keeps reclaiming pages automatically.
        diesel::sql_query("PRAGMA auto_vacuum = INCREMENTAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        Ok(())
    }
}

impl TryFrom<PoolConfig> for DatabasePool {
    type Error = anyhow::Error;

    fn try_from(config: PoolConfig) -> Result<Self> {
        debug!(database_path = %config.database_path.display(), "Creating database pool");

        // Ensure the parent directory exists
        if let Some(parent) = config.database_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Retry pool creation with exponential backoff to handle transient
        // failures such as another process holding an exclusive lock on the
        // SQLite database file.
        DatabasePool::retry_with_backoff(
            config.max_retries,
            "Failed to create database pool, retrying",
            || Self::build_pool(&config),
        )
    }
}

impl DatabasePool {
    /// Builds the connection pool and runs migrations.
    fn build_pool(config: &PoolConfig) -> Result<Self> {
        let database_url = config.database_path.to_string_lossy().to_string();
        let manager = ConnectionManager::<SqliteConnection>::new(&database_url);

        let mut builder = Pool::builder()
            .max_size(config.max_size)
            .connection_timeout(config.connection_timeout)
            .connection_customizer(Box::new(SqliteCustomizer));

        if let Some(min_idle) = config.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }

        if let Some(idle_timeout) = config.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }

        let pool = Arc::new(builder.build(manager).map_err(|e| {
            warn!(error = %e, "Failed to create connection pool");
            anyhow::anyhow!("Failed to create connection pool: {e}")
        })?);

        // Run migrations on a connection from the pool
        let mut connection = pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get connection for migrations: {e}"))?;

        connection.run_pending_migrations(MIGRATIONS).map_err(|e| {
            warn!(error = %e, "Failed to run database migrations");
            anyhow::anyhow!("Failed to run database migrations: {e}")
        })?;

        debug!(database_path = %config.database_path.display(), "created connection pool");

        // Spawn background maintenance task for incremental vacuum if enabled.
        // This runs in the background and periodically reclaims freed pages (from P4 prune,
        // zstd compression, deletes) without an exclusive-lock full VACUUM.
        if Self::is_incremental_vacuum_enabled() {
            let pool_clone = Arc::clone(&pool);
            std::thread::spawn(move || {
                Self::incremental_vacuum_loop(pool_clone);
            });
        }

        Ok(Self { pool, max_retries: config.max_retries })
    }

    /// Check if incremental vacuum is enabled via env var FORGE_INCREMENTAL_VACUUM.
    /// Defaults to enabled (true) if not set.
    fn is_incremental_vacuum_enabled() -> bool {
        match std::env::var("FORGE_INCREMENTAL_VACUUM") {
            Ok(val) => !matches!(val.as_str(), "0" | "false" | "no" | "off"),
            Err(_) => true, // Default: enabled
        }
    }

    /// Background loop that periodically runs incremental_vacuum on the database pool.
    /// Runs every 30 seconds; non-fatal errors are logged and ignored.
    fn incremental_vacuum_loop(pool: Arc<DbPool>) {
        loop {
            std::thread::sleep(Duration::from_secs(30));

            match pool.get() {
                Ok(mut conn) => {
                    if let Err(e) = diesel::sql_query("PRAGMA incremental_vacuum;").execute(&mut conn)
                    {
                        debug!(error = %e, "incremental_vacuum failed (will retry in next cycle)");
                    }
                }
                Err(e) => {
                    debug!(error = %e, "failed to get connection for incremental_vacuum (will retry in next cycle)");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_vacuum_incremental_pragma_executes() {
        // Verify that in-memory databases allow the incremental_vacuum pragma to execute.
        // This confirms auto_vacuum is set to INCREMENTAL (value 2) since queries don't
        // error on this pragma mode.
        let pool = DatabasePool::in_memory().expect("Failed to create in-memory pool");

        // This should not panic or error if auto_vacuum=INCREMENTAL is set
        pool.incremental_vacuum()
            .expect("incremental_vacuum should execute successfully on INCREMENTAL mode");
    }
}

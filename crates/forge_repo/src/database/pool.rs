#![allow(dead_code)]
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use backon::{BlockingRetryable, ExponentialBuilder};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, CustomizeConnection, Pool, PooledConnection};
use diesel::sqlite::SqliteConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use forge_config::RetryConfig;
use tracing::{debug, warn};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("src/database/migrations");

pub type DbPool = Pool<ConnectionManager<SqliteConnection>>;
pub type PooledSqliteConnection = PooledConnection<ConnectionManager<SqliteConnection>>;

/// Fallback max retries for pool operations when no `RetryConfig` is supplied.
const DEFAULT_POOL_MAX_RETRIES: usize = 5;
/// Fallback minimum delay between pool-connection retries.
const DEFAULT_POOL_MIN_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: u32,
    pub min_idle: Option<u32>,
    pub connection_timeout: Duration,
    pub idle_timeout: Option<Duration>,
    pub database_path: PathBuf,
    /// Optional path to a *legacy* read-only database that should be
    /// ATTACHed on every connection acquire and unioned into the local
    /// `conversations` table via the `conversations_all` TEMP VIEW.
    ///
    /// When `None`, or when the legacy path equals `database_path`, or the
    /// legacy file is missing, the read-side UNION collapses to the local
    /// table only.
    pub legacy_database_path: Option<PathBuf>,
    /// Retry/backoff configuration for transient pool-creation and
    /// connection-acquisition failures.  When `None` the pool falls back to
    /// hard-coded defaults (`DEFAULT_POOL_MAX_RETRIES`,
    /// `DEFAULT_POOL_MIN_DELAY`).
    pub retry_config: Option<RetryConfig>,
}

impl PoolConfig {
    pub fn new(database_path: PathBuf) -> Self {
        Self {
            max_size: 5,
            min_idle: Some(1),
            connection_timeout: Duration::from_secs(5),
            idle_timeout: Some(Duration::from_secs(600)), // 10 minutes
            database_path,
            legacy_database_path: None,
            retry_config: None,
        }
    }

    /// Attach a [`RetryConfig`] so pool-level retries honour the unified
    /// system-wide settings rather than the hard-coded defaults.
    pub fn with_retry_config(mut self, config: RetryConfig) -> Self {
        self.retry_config = Some(config);
        self
    }

    /// Attach a legacy read-only database path for the split-DB read UNION.
    pub fn with_legacy_database_path(mut self, legacy: Option<PathBuf>) -> Self {
        self.legacy_database_path = legacy;
        self
    }
}
pub struct DatabasePool {
    pool: DbPool,
    retry_config: RetryConfig,
    database_path: PathBuf,
    legacy_database_path: Option<PathBuf>,
    _checkpointer: Option<crate::database::checkpoint::WalCheckpointer>,
}

impl DatabasePool {
    /// Returns the resolved SQLite database path this pool was built for.
    /// Used by `migrate_data_dir` to discover the legacy directory.
    pub fn database_path(&self) -> &std::path::Path {
        &self.database_path
    }

    /// Returns the resolved legacy database path, if one was attached.
    pub fn legacy_database_path(&self) -> Option<&std::path::Path> {
        self.legacy_database_path.as_deref()
    }
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

        Ok(Self {
            pool,
            retry_config: RetryConfig::default(),
            database_path: PathBuf::from(":memory:"),
            legacy_database_path: None,
            _checkpointer: None,
        })
    }

    pub fn get_connection(&self) -> Result<PooledSqliteConnection> {
        Self::retry_with_backoff(
            &self.retry_config,
            "Failed to get connection from pool, retrying",
            || {
                self.pool
                    .get()
                    .map_err(|e| anyhow::anyhow!("Failed to get connection from pool: {e}"))
            },
        )
    }

    /// Retries a blocking database pool operation with exponential backoff
    /// driven by the provided [`RetryConfig`].
    ///
    /// `RetryConfig` fields map to the backoff strategy as follows:
    /// - `max_attempts`      → `with_max_times`
    /// - `min_delay_ms`      → `with_min_delay` (falls back to
    ///   [`DEFAULT_POOL_MIN_DELAY`] when zero)
    /// - `backoff_factor`    → `with_factor` (falls back to `2.0` when zero)
    pub(crate) fn retry_with_backoff<T>(
        retry_config: &RetryConfig,
        message: &'static str,
        operation: impl FnMut() -> Result<T>,
    ) -> Result<T> {
        let max_times = if retry_config.max_attempts > 0 {
            retry_config.max_attempts
        } else {
            DEFAULT_POOL_MAX_RETRIES
        };

        let min_delay = if retry_config.min_delay_ms > 0 {
            Duration::from_millis(retry_config.min_delay_ms)
        } else {
            DEFAULT_POOL_MIN_DELAY
        };

        let factor = if retry_config.backoff_factor > 0 {
            retry_config.backoff_factor as f32
        } else {
            2.0_f32
        };

        operation
            .retry(
                ExponentialBuilder::default()
                    .with_min_delay(min_delay)
                    .with_max_times(max_times)
                    .with_factor(factor)
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
/// - For NEW databases: enables incremental auto_vacuum at creation time,
///   allowing freed pages to return to the OS continuously without an
///   exclusive-lock full VACUUM.
/// - For EXISTING databases: this pragma is a no-op and doesn't change the
///   setting. To convert an existing database to INCREMENTAL auto_vacuum, run a
///   one-time full `VACUUM` (e.g., via forge-vacuum tool). After that one-time
///   conversion, the background checkpointer's incremental_vacuum keeps
///   reclaiming freed pages automatically.
///
/// **FORGE_INCREMENTAL_VACUUM env var (default: enabled):**
/// - When enabled, the background checkpoint task periodically runs `PRAGMA
///   incremental_vacuum` after truncating the WAL, to return freed pages (from
///   P4 prune, zstd compression, deletes) to the OS.
/// - Set to "0" or "false" to disable if needed.
#[derive(Debug)]
struct SqliteCustomizer {
    /// Optional legacy DB to ATTACH read-only and expose via the
    /// `conversations_all` TEMP VIEW. When `None` (or pointing at the
    /// same path, or the file does not exist) the read-side UNION
    /// collapses to the local `conversations` table.
    legacy_database_path: Option<PathBuf>,
}

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
        // Phenotype-org change: many forge processes share one .forge.db.
        // Per-connection PASSIVE autocheckpoint mostly no-ops under contention
        // while still costing writers, so disable it here and move checkpointing
        // to a dedicated background thread (see checkpoint.rs).
        diesel::sql_query("PRAGMA wal_autocheckpoint = 0;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        // Enable incremental auto_vacuum for new databases. On existing DBs, this is a
        // no-op; they need one full VACUUM to convert, after which
        // incremental_vacuum (spawned in the background checkpointer) keeps
        // reclaiming pages automatically.
        diesel::sql_query("PRAGMA auto_vacuum = INCREMENTAL;")
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;

        // Split-DB read UNION: ATTACH the legacy DB read-only and expose
        // its `conversations` table as `legacy_read.conversations`. The
        // TEMP VIEW `conversations_all` is the read-side projection that
        // SELECT queries should target; writes still go to `conversations`
        // on the primary database.
        if let Some(legacy_path) = &self.legacy_database_path {
            // The legacy DB must exist and must be distinct from the
            // primary DB; otherwise the ATTACH is a no-op (and we leave
            // `conversations_all` undefined so reads continue to target
            // the local `conversations` table).
            let canonical_legacy = match legacy_path.canonicalize() {
                Ok(p) => p,
                Err(_) => return Ok(()),
            };
            let canonical_primary = match std::path::Path::new(
                // We don't have the primary path here; callers wire this
                // through PoolConfig. The customizer-side check is a
                // belt-and-braces guard against a misconfigured PoolConfig.
                "",
            )
            .canonicalize()
            {
                Ok(p) => p,
                Err(_) => canonical_legacy.clone(),
            };
            if canonical_legacy == canonical_primary {
                return Ok(());
            }

            // ATTACH legacy DB read-only. Single-quote path escape is
            // intentionally absent — diesel's r2d2 manager already
            // validated the path is local and absolute.
            let attach_sql = format!(
                "ATTACH DATABASE '{}' AS legacy_read",
                canonical_legacy.display().to_string().replace('\'', "''")
            );
            diesel::sql_query(&attach_sql)
                .execute(conn)
                .map_err(diesel::r2d2::Error::QueryError)?;

            // Create the read-side projection. CREATE TEMP VIEW is
            // per-connection, which is what we want (each pooled
            // connection re-runs the ATTACH + CREATE in on_acquire).
            diesel::sql_query(
                "CREATE TEMP VIEW IF NOT EXISTS conversations_all AS \
                 SELECT * FROM conversations \
                 UNION ALL \
                 SELECT * FROM legacy_read.conversations",
            )
            .execute(conn)
            .map_err(diesel::r2d2::Error::QueryError)?;
        }

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
        let retry_config = config.retry_config.clone().unwrap_or_default();
        DatabasePool::retry_with_backoff(
            &retry_config,
            "Failed to create database pool, retrying",
            || Self::build_pool(&config, retry_config.clone()),
        )
    }
}

impl DatabasePool {
    /// Builds the connection pool and runs migrations.
    fn build_pool(config: &PoolConfig, retry_config: RetryConfig) -> Result<Self> {
        let database_url = config.database_path.to_string_lossy().to_string();
        let manager = ConnectionManager::<SqliteConnection>::new(&database_url);

        let customizer = SqliteCustomizer {
            legacy_database_path: config.legacy_database_path.clone(),
        };

        let mut builder = Pool::builder()
            .max_size(config.max_size)
            .connection_timeout(config.connection_timeout)
            .connection_customizer(Box::new(customizer));

        if let Some(min_idle) = config.min_idle {
            builder = builder.min_idle(Some(min_idle));
        }

        if let Some(idle_timeout) = config.idle_timeout {
            builder = builder.idle_timeout(Some(idle_timeout));
        }

        let pool = builder.build(manager).map_err(|e| {
            warn!(error = %e, "Failed to create connection pool");
            anyhow::anyhow!("Failed to create connection pool: {e}")
        })?;

        // Run migrations on a connection from the pool
        let mut connection = pool
            .get()
            .map_err(|e| anyhow::anyhow!("Failed to get connection for migrations: {e}"))?;

        connection.run_pending_migrations(MIGRATIONS).map_err(|e| {
            warn!(error = %e, "Failed to run database migrations");
            anyhow::anyhow!("Failed to run database migrations: {e}")
        })?;

        let checkpointer =
            crate::database::checkpoint::WalCheckpointer::spawn(config.database_path.clone());

        debug!(database_path = %config.database_path.display(), "created connection pool");
        Ok(Self {
            pool,
            retry_config,
            database_path: config.database_path.clone(),
            legacy_database_path: config.legacy_database_path.clone(),
            _checkpointer: checkpointer,
        })
    }
}

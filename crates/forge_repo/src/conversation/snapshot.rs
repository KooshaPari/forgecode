//! Read-only Forge conversation snapshots for the HeliosLite boundary.
//!
//! This module deliberately does not use [`crate::database::DatabasePool`].
//! That pool is a writable, migration-running runtime abstraction and is
//! therefore unsafe for reading the standard Forge database as a source.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::codec;
use anyhow::{Context, Result, bail};
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Binary, Integer, Nullable, Text, Timestamp};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Version of the HeliosLite Forge-session snapshot contract.
pub const SNAPSHOT_CONTRACT_VERSION: &str = "helioslite-forge-session-v2";
pub const SUPPORTED_SCHEMA_VERSION: &str = "forge-conversations-v3";
const IMPORTER_VERSION: &str = env!("CARGO_PKG_VERSION");
const REQUIRED_COLUMNS: [&str; 18] = [
    "conversation_id",
    "title",
    "workspace_id",
    "context",
    "context_zstd",
    "is_compressed",
    "hidden",
    "parent_id",
    "source",
    "cwd",
    "message_count",
    "created_at",
    "updated_at",
    "intent_state",
    "metrics",
    "extracted_at",
    "memory_id",
    "intent_hash",
];

/// A conversation row copied without interpreting or rewriting its payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForgeSnapshotRow {
    /// Stable Forge conversation key. The obsolete `id` column is never used.
    pub conversation_id: String,
    pub title: Option<String>,
    pub workspace_id: i64,
    pub context: Option<String>,
    pub context_zstd: Option<Vec<u8>>,
    pub is_compressed: i32,
    pub hidden: i32,
    pub parent_id: Option<String>,
    pub source: Option<String>,
    pub cwd: Option<String>,
    pub message_count: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub intent_state: String,
    pub metrics: Option<String>,
    pub extracted_at: Option<NaiveDateTime>,
    pub memory_id: Option<String>,
    pub intent_hash: Option<String>,
}

/// Provenance and integrity information for a Forge snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForgeSnapshotManifest {
    pub contract_version: String,
    pub importer_version: String,
    pub source_path: String,
    pub source_sha256: String,
    pub source_size: u64,
    pub source_modified_unix_ms: Option<u128>,
    pub source_before_sha256: String,
    pub source_before_size: u64,
    pub source_before_modified_unix_ms: Option<u128>,
    pub source_after_sha256: String,
    pub source_after_size: u64,
    pub source_after_modified_unix_ms: Option<u128>,
    pub source_schema_fingerprint: String,
    pub source_schema_version: String,
    pub required_columns: Vec<String>,
    pub export_started_at_unix_ms: u128,
    pub export_completed_at_unix_ms: u128,
    pub exported_at_unix_ms: u128,
    pub row_count: usize,
    pub content_sha256: String,
    pub row_digest: String,
    pub id_digest: String,
    pub source_read_only: bool,
    pub source_unchanged: bool,
    pub status: String,
    pub destination_path: Option<String>,
    pub destination_content_sha256: Option<String>,
}

/// A serializable snapshot and its manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForgeSnapshot {
    pub manifest: ForgeSnapshotManifest,
    pub rows: Vec<ForgeSnapshotRow>,
}

#[derive(Debug, QueryableByName)]
struct SchemaRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    column_type: String,
    #[diesel(sql_type = Integer)]
    not_null: i32,
    #[diesel(sql_type = Nullable<Text>)]
    default_value: Option<String>,
    #[diesel(sql_type = Integer)]
    primary_key: i32,
}

#[derive(Debug, QueryableByName)]
struct ConversationRow {
    #[diesel(sql_type = Text)]
    conversation_id: String,
    #[diesel(sql_type = Nullable<Text>)]
    title: Option<String>,
    #[diesel(sql_type = BigInt)]
    workspace_id: i64,
    #[diesel(sql_type = Nullable<Text>)]
    context: Option<String>,
    #[diesel(sql_type = Nullable<Binary>)]
    context_zstd: Option<Vec<u8>>,
    #[diesel(sql_type = Integer)]
    is_compressed: i32,
    #[diesel(sql_type = Integer)]
    hidden: i32,
    #[diesel(sql_type = Nullable<Text>)]
    parent_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    source: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    cwd: Option<String>,
    #[diesel(sql_type = Nullable<Integer>)]
    message_count: Option<i32>,
    #[diesel(sql_type = Timestamp)]
    created_at: NaiveDateTime,
    #[diesel(sql_type = Nullable<Timestamp>)]
    updated_at: Option<NaiveDateTime>,
    #[diesel(sql_type = Text)]
    intent_state: String,
    #[diesel(sql_type = Nullable<Text>)]
    metrics: Option<String>,
    #[diesel(sql_type = Nullable<Timestamp>)]
    extracted_at: Option<NaiveDateTime>,
    #[diesel(sql_type = Nullable<Text>)]
    memory_id: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    intent_hash: Option<String>,
}

#[derive(Debug, QueryableByName)]
struct QueryOnlyRow {
    #[diesel(sql_type = Integer)]
    query_only: i32,
}

/// Export all conversations from an existing Forge SQLite file without any
/// write-capable connection or migration side effect.
///
/// The source is opened with SQLite `mode=ro&immutable=1`; a pre/post file
/// fingerprint check additionally fails closed if another process changes it.
pub fn export_forge_snapshot(source: &Path) -> Result<ForgeSnapshot> {
    let export_started_at_unix_ms = now_unix_ms();
    let source = source
        .canonicalize()
        .with_context(|| format!("canonicalize Forge source {}", source.display()))?;
    if !source.is_file() {
        bail!("Forge source is not a regular file: {}", source.display());
    }
    reject_sqlite_sidecars(&source)?;

    let before = fingerprint(&source)?;
    let uri = sqlite_read_only_uri(&source)?;
    let mut connection = diesel::sqlite::SqliteConnection::establish(&uri)
        .with_context(|| format!("open Forge source read-only: {}", source.display()))?;
    diesel::sql_query("PRAGMA query_only = ON")
        .execute(&mut connection)
        .context("enable SQLite query_only mode")?;
    let query_only = diesel::sql_query("PRAGMA query_only")
        .get_result::<QueryOnlyRow>(&mut connection)
        .context("verify SQLite query_only mode")?
        .query_only;
    if query_only != 1 {
        bail!("SQLite source did not enter query_only mode");
    }

    let schema = read_schema(&mut connection)?;
    validate_schema(&schema)?;
    let schema_fingerprint = schema_fingerprint(&schema);
    let rows: Vec<ForgeSnapshotRow> = diesel::sql_query(
        "SELECT conversation_id, title, workspace_id, context, context_zstd, \
         is_compressed, hidden, parent_id, source, cwd, message_count, \
         created_at, updated_at, intent_state, metrics, extracted_at, memory_id, intent_hash \
         FROM conversations ORDER BY conversation_id",
    )
    .load::<ConversationRow>(&mut connection)
    .context("read Forge conversations")?
    .into_iter()
    .map(Into::into)
    .collect();
    drop(connection);

    let after = fingerprint(&source)?;
    if before != after {
        bail!("Forge source changed during read; refusing snapshot");
    }
    validate_rows(&rows)?;
    let content_sha256 = content_digest(&rows)?;
    let row_digest = content_sha256.clone();
    let id_digest = id_digest(&rows);
    let export_completed_at_unix_ms = now_unix_ms();
    Ok(ForgeSnapshot {
        manifest: ForgeSnapshotManifest {
            contract_version: SNAPSHOT_CONTRACT_VERSION.to_string(),
            importer_version: IMPORTER_VERSION.to_string(),
            source_path: source.display().to_string(),
            source_sha256: after.sha256.clone(),
            source_size: after.size,
            source_modified_unix_ms: after.modified_unix_ms,
            source_before_sha256: before.sha256,
            source_before_size: before.size,
            source_before_modified_unix_ms: before.modified_unix_ms,
            source_after_sha256: after.sha256,
            source_after_size: after.size,
            source_after_modified_unix_ms: after.modified_unix_ms,
            source_schema_fingerprint: schema_fingerprint.clone(),
            source_schema_version: SUPPORTED_SCHEMA_VERSION.to_string(),
            required_columns: REQUIRED_COLUMNS.iter().map(|c| (*c).to_string()).collect(),
            export_started_at_unix_ms,
            export_completed_at_unix_ms,
            exported_at_unix_ms: export_completed_at_unix_ms,
            row_count: rows.len(),
            content_sha256,
            row_digest,
            id_digest,
            source_read_only: true,
            source_unchanged: true,
            status: "exported".to_string(),
            destination_path: None,
            destination_content_sha256: None,
        },
        rows,
    })
}

/// Publish a snapshot bundle into a new destination directory atomically.
///
/// The destination must not already exist. A staging directory is created next
/// to it, fsynced, and renamed only after both JSON files are complete.
pub fn publish_snapshot_atomic(snapshot: &ForgeSnapshot, destination: &Path) -> Result<()> {
    if destination.exists() {
        bail!(
            "snapshot destination already exists: {}",
            destination.display()
        );
    }
    let parent = destination
        .parent()
        .context("snapshot destination must have a parent directory")?;
    fs::create_dir_all(parent)
        .with_context(|| format!("create snapshot parent {}", parent.display()))?;
    let stage = unique_staging_dir(parent, destination.file_name().unwrap_or_default())?;
    fs::create_dir(&stage)
        .with_context(|| format!("create snapshot staging directory {}", stage.display()))?;
    let result = (|| -> Result<()> {
        write_synced(
            &stage.join("snapshot.json"),
            &serde_json::to_vec_pretty(snapshot)?,
        )?;
        write_synced(
            &stage.join("manifest.json"),
            &serde_json::to_vec_pretty(&snapshot.manifest)?,
        )?;
        File::open(&stage)?.sync_all()?;
        fs::rename(&stage, destination)
            .with_context(|| format!("publish snapshot {}", destination.display()))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result
}

impl From<ConversationRow> for ForgeSnapshotRow {
    fn from(row: ConversationRow) -> Self {
        Self {
            conversation_id: row.conversation_id,
            title: row.title,
            workspace_id: row.workspace_id,
            context: row.context,
            context_zstd: row.context_zstd,
            is_compressed: row.is_compressed,
            hidden: row.hidden,
            parent_id: row.parent_id,
            source: row.source,
            cwd: row.cwd,
            message_count: row.message_count,
            created_at: row.created_at,
            updated_at: row.updated_at,
            intent_state: row.intent_state,
            metrics: row.metrics,
            extracted_at: row.extracted_at,
            memory_id: row.memory_id,
            intent_hash: row.intent_hash,
        }
    }
}

#[derive(Debug, PartialEq)]
struct Fingerprint {
    sha256: String,
    size: u64,
    modified_unix_ms: Option<u128>,
}

fn fingerprint(path: &Path) -> Result<Fingerprint> {
    let metadata = fs::metadata(path)?;
    let mut file = BufReader::new(File::open(path)?);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    let mut size = 0_u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        size += read as u64;
        hasher.update(&buffer[..read]);
    }
    let modified_unix_ms = metadata.modified().ok().and_then(|value| {
        value
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_millis())
    });
    Ok(Fingerprint {
        sha256: hex::encode(hasher.finalize()),
        size,
        modified_unix_ms,
    })
}

fn sqlite_read_only_uri(path: &Path) -> Result<String> {
    let path = path.to_str().context("Forge source path is not UTF-8")?;
    let encoded = path
        .replace('%', "%25")
        .replace('?', "%3F")
        .replace('#', "%23");
    Ok(format!("file:{encoded}?mode=ro&immutable=1"))
}

fn reject_sqlite_sidecars(source: &Path) -> Result<()> {
    for suffix in ["-wal", "-shm", "-journal"] {
        let sidecar = PathBuf::from(format!("{}{suffix}", source.display()));
        match fs::symlink_metadata(&sidecar) {
            Ok(_) => bail!(
                "SQLite sidecar is present beside Forge source; refusing snapshot: {}",
                sidecar.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("inspect SQLite sidecar {}", sidecar.display()));
            }
        }
    }
    Ok(())
}

fn read_schema(connection: &mut diesel::sqlite::SqliteConnection) -> Result<Vec<SchemaRow>> {
    diesel::sql_query(
        "SELECT name, type AS column_type, \"notnull\" AS not_null, \
         \"dflt_value\" AS default_value, \"pk\" AS primary_key \
         FROM pragma_table_info('conversations') ORDER BY cid",
    )
    .load(connection)
    .context("inspect conversations schema")
}

fn validate_schema(schema: &[SchemaRow]) -> Result<()> {
    let columns: BTreeMap<&str, &SchemaRow> =
        schema.iter().map(|row| (row.name.as_str(), row)).collect();
    let missing: Vec<&str> = REQUIRED_COLUMNS
        .iter()
        .copied()
        .filter(|column| !columns.contains_key(column))
        .collect();
    if !missing.is_empty() {
        bail!(
            "unsupported Forge conversations schema; missing columns: {}",
            missing.join(", ")
        );
    }
    let unknown: Vec<&str> = columns
        .keys()
        .copied()
        .filter(|column| !REQUIRED_COLUMNS.contains(column))
        .collect();
    if !unknown.is_empty() {
        bail!(
            "unsupported Forge conversations schema; unknown columns: {}",
            unknown.join(", ")
        );
    }
    let expected_types = [
        ("conversation_id", "TEXT"),
        ("title", "TEXT"),
        ("workspace_id", "BIGINT"),
        ("context", "TEXT"),
        ("created_at", "TIMESTAMP"),
        ("updated_at", "TIMESTAMP"),
        ("metrics", "TEXT"),
        ("parent_id", "TEXT"),
        ("source", "TEXT"),
        ("cwd", "TEXT"),
        ("message_count", "INTEGER"),
        ("intent_state", "TEXT"),
        ("extracted_at", "TIMESTAMP"),
        ("memory_id", "TEXT"),
        ("intent_hash", "TEXT"),
        ("context_zstd", "BLOB"),
        ("is_compressed", "INTEGER"),
        ("hidden", "INTEGER"),
    ];
    for (name, expected) in expected_types {
        let actual = columns
            .get(name)
            .with_context(|| format!("schema column disappeared while validating: {name}"))?;
        if !actual.column_type.eq_ignore_ascii_case(expected) {
            bail!(
                "unsupported Forge conversations schema; {name} has type {}, expected {expected}",
                actual.column_type
            );
        }
    }
    if columns
        .get("conversation_id")
        .is_some_and(|row| row.primary_key != 1)
    {
        bail!("unsupported Forge conversations schema; conversation_id is not the primary key");
    }
    Ok(())
}

fn schema_fingerprint(schema: &[SchemaRow]) -> String {
    let mut rows: Vec<String> = schema
        .iter()
        .map(|row| {
            format!(
                "{}|{}|{}|{}|{}",
                row.name,
                row.column_type,
                row.not_null,
                row.primary_key,
                row.default_value.as_deref().unwrap_or("")
            )
        })
        .collect();
    rows.sort();
    let mut hasher = Sha256::new();
    hasher.update(rows.join("\n"));
    hex::encode(hasher.finalize())
}

fn content_digest(rows: &[ForgeSnapshotRow]) -> Result<String> {
    let bytes = serde_json::to_vec(rows).context("serialize snapshot rows for digest")?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn id_digest(rows: &[ForgeSnapshotRow]) -> String {
    let ids = rows
        .iter()
        .map(|row| row.conversation_id.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    hex::encode(Sha256::digest(ids.as_bytes()))
}

fn validate_rows(rows: &[ForgeSnapshotRow]) -> Result<()> {
    for row in rows {
        if row.is_compressed != 0 && row.is_compressed != 1 {
            bail!(
                "invalid is_compressed flag for conversation {}",
                row.conversation_id
            );
        }
        if row.is_compressed == 1 {
            let compressed = row.context_zstd.as_deref().with_context(|| {
                format!(
                    "compressed conversation {} has no context_zstd",
                    row.conversation_id
                )
            })?;
            codec::decompress(compressed).with_context(|| {
                format!(
                    "invalid compressed context for conversation {}",
                    row.conversation_id
                )
            })?;
        }
    }
    Ok(())
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn unique_staging_dir(parent: &Path, name: &std::ffi::OsStr) -> Result<PathBuf> {
    for attempt in 0..100_u32 {
        let candidate = parent.join(format!(
            ".{}.staging-{}-{}",
            name.to_string_lossy(),
            std::process::id(),
            attempt
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    bail!("unable to allocate snapshot staging directory")
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{Connection, connection::SimpleConnection};
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    fn fixture(path: &Path, include_id_only: bool) -> Result<()> {
        let mut connection = diesel::sqlite::SqliteConnection::establish(path.to_str().unwrap())?;
        diesel::sql_query(
            "CREATE TABLE conversations (
                conversation_id TEXT PRIMARY KEY NOT NULL,
                title TEXT, workspace_id BIGINT NOT NULL, context TEXT,
                context_zstd BLOB, is_compressed INTEGER NOT NULL DEFAULT 0,
                hidden INTEGER NOT NULL DEFAULT 0, parent_id TEXT, source TEXT,
                cwd TEXT, message_count INTEGER, created_at TIMESTAMP NOT NULL,
                updated_at TIMESTAMP, intent_state TEXT NOT NULL DEFAULT '{}',
                metrics TEXT, extracted_at TIMESTAMP, memory_id TEXT, intent_hash TEXT
            )",
        )
        .execute(&mut connection)?;
        diesel::sql_query(
            "INSERT INTO conversations (conversation_id,title,workspace_id,context,is_compressed,hidden,created_at,intent_state)
             VALUES ('root','Root',7,'plain',0,0,'2026-01-01 00:00:00','{}')",
        )
        .execute(&mut connection)?;
        let compressed = codec::compress("compressed")?;
        diesel::sql_query(
            "INSERT INTO conversations (conversation_id,title,workspace_id,context_zstd,is_compressed,hidden,parent_id,created_at,intent_state)
             VALUES ('child',NULL,7,?,1,1,'root','2026-01-01 00:00:01','{}')",
        )
        .bind::<Binary, _>(compressed)
        .execute(&mut connection)?;
        if include_id_only {
            diesel::sql_query("ALTER TABLE conversations RENAME TO old_conversations")
                .execute(&mut connection)?;
            diesel::sql_query(
                "CREATE TABLE conversations (id TEXT PRIMARY KEY, workspace_id BIGINT, context TEXT, created_at TIMESTAMP)",
            )
            .execute(&mut connection)?;
            diesel::sql_query(
                "INSERT INTO conversations VALUES ('wrong',7,'bad','2026-01-01 00:00:00')",
            )
            .execute(&mut connection)?;
        }
        connection
            .batch_execute("PRAGMA wal_checkpoint(TRUNCATE)")
            .ok();
        Ok(())
    }

    #[test]
    fn exports_live_columns_without_mutating_source() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("forge.db");
        fixture(&source, false)?;
        let before = fingerprint(&source)?;
        let actual = export_forge_snapshot(&source)?;
        let after = fingerprint(&source)?;
        let expected = vec!["child".to_string(), "root".to_string()];
        let ids = actual
            .rows
            .iter()
            .map(|row| row.conversation_id.clone())
            .collect::<Vec<_>>();
        assert_eq!(ids, expected);
        assert_eq!(actual.manifest.row_count, 2);
        assert_eq!(actual.manifest.source_sha256, before.sha256);
        assert_eq!(before, after);
        assert!(
            actual
                .rows
                .iter()
                .any(|row| row.is_compressed == 1 && row.hidden == 1)
        );
        Ok(())
    }

    #[test]
    fn rejects_schema_without_conversation_id() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("legacy.db");
        fixture(&source, true)?;
        let error = export_forge_snapshot(&source).expect_err("legacy id schema must fail");
        assert!(error.to_string().contains("conversation_id"));
        Ok(())
    }

    #[test]
    fn rejects_live_sqlite_sidecars_before_reading_source() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("forge.db");
        fixture(&source, false)?;

        for suffix in ["-wal", "-shm"] {
            let sidecar = PathBuf::from(format!("{}{}", source.display(), suffix));
            File::create(&sidecar)?;
            let error =
                export_forge_snapshot(&source).expect_err("live SQLite sidecar must fail closed");
            assert!(error.to_string().contains("SQLite sidecar"));
            fs::remove_file(sidecar)?;
        }

        Ok(())
    }

    #[test]
    fn publishes_snapshot_as_atomic_bundle() -> Result<()> {
        let dir = tempdir()?;
        let source = dir.path().join("forge.db");
        fixture(&source, false)?;
        let snapshot = export_forge_snapshot(&source)?;
        let destination = dir.path().join("sessions").join("snapshot");
        publish_snapshot_atomic(&snapshot, &destination)?;
        assert!(destination.join("snapshot.json").is_file());
        assert!(destination.join("manifest.json").is_file());
        let duplicate =
            publish_snapshot_atomic(&snapshot, &destination).expect_err("must not overwrite");
        assert!(duplicate.to_string().contains("already exists"));
        Ok(())
    }
}

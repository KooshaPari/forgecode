use chrono::Utc;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLabelRequest {
    pub name: String,
    #[serde(default = "default_color")]
    pub color: String,
}

fn default_color() -> String {
    "#58a6ff".to_string()
}

// ── Database Operations ───────────────────────────────────────────────────

pub fn init_labels_table(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS labels (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            color       TEXT NOT NULL DEFAULT '#58a6ff',
            created_at  TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

pub fn create_label(conn: &Connection, req: CreateLabelRequest) -> SqlResult<Label> {
    let now = Utc::now().to_rfc3339();
    let label = Label {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        color: req.color,
        created_at: now.clone(),
    };
    conn.execute(
        "INSERT INTO labels (id, name, color, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![label.id, label.name, label.color, label.created_at],
    )?;
    Ok(label)
}

pub fn list_labels(conn: &Connection) -> SqlResult<Vec<Label>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, color, created_at FROM labels ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Label {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn delete_label(conn: &Connection, id: &str) -> SqlResult<bool> {
    let rows = conn.execute("DELETE FROM labels WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

use chrono::Utc;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub priority: String,
    pub assignee: String,
    pub labels: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default)]
    pub assignee: String,
    #[serde(default)]
    pub labels: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub labels: Option<String>,
}

fn default_status() -> String {
    "Backlog".to_string()
}

fn default_priority() -> String {
    "Medium".to_string()
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open (or create) the SQLite database and run migrations.
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.init_db()?;
        Ok(db)
    }

    /// Get a reference to the underlying connection (for external module use).
    pub fn get_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    fn init_db(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS issues (
                id          TEXT PRIMARY KEY,
                title       TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                status      TEXT NOT NULL DEFAULT 'Backlog',
                priority    TEXT NOT NULL DEFAULT 'Medium',
                assignee    TEXT NOT NULL DEFAULT '',
                labels      TEXT NOT NULL DEFAULT '',
                created_at  TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
            CREATE INDEX IF NOT EXISTS idx_issues_priority ON issues(priority);
            ",
        )?;

        // Initialize sprint tables
        crate::sprint::init_sprints_table(&conn)?;

        // Initialize labels table
        crate::labels::init_labels_table(&conn)?;

        Ok(())
    }

    pub fn create_issue(&self, req: CreateIssueRequest) -> SqlResult<Issue> {
        let now = Utc::now().to_rfc3339();
        let issue = Issue {
            id: Uuid::new_v4().to_string(),
            title: req.title,
            description: req.description,
            status: req.status,
            priority: req.priority,
            assignee: req.assignee,
            labels: req.labels,
            created_at: now.clone(),
            updated_at: now,
        };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO issues (id, title, description, status, priority, assignee, labels, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                issue.id,
                issue.title,
                issue.description,
                issue.status,
                issue.priority,
                issue.assignee,
                issue.labels,
                issue.created_at,
                issue.updated_at,
            ],
        )?;
        Ok(issue)
    }

    pub fn list_issues(&self) -> SqlResult<Vec<Issue>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, description, status, priority, assignee, labels, created_at, updated_at
             FROM issues ORDER BY created_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Issue {
                id: row.get(0)?,
                title: row.get(1)?,
                description: row.get(2)?,
                status: row.get(3)?,
                priority: row.get(4)?,
                assignee: row.get(5)?,
                labels: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        })?;
        rows.collect()
    }

    pub fn update_issue(&self, req: UpdateIssueRequest) -> SqlResult<Issue> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Fetch current issue first
        let current: Issue = conn.query_row(
            "SELECT id, title, description, status, priority, assignee, labels, created_at, updated_at
             FROM issues WHERE id = ?1",
            params![req.id],
            |row| {
                Ok(Issue {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    description: row.get(2)?,
                    status: row.get(3)?,
                    priority: row.get(4)?,
                    assignee: row.get(5)?,
                    labels: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            },
        )?;

        let updated = Issue {
            id: current.id,
            title: req.title.unwrap_or(current.title),
            description: req.description.unwrap_or(current.description),
            status: req.status.unwrap_or(current.status),
            priority: req.priority.unwrap_or(current.priority),
            assignee: req.assignee.unwrap_or(current.assignee),
            labels: req.labels.unwrap_or(current.labels),
            created_at: current.created_at,
            updated_at: now,
        };

        conn.execute(
            "UPDATE issues SET title = ?1, description = ?2, status = ?3, priority = ?4,
             assignee = ?5, labels = ?6, updated_at = ?7
             WHERE id = ?8",
            params![
                updated.title,
                updated.description,
                updated.status,
                updated.priority,
                updated.assignee,
                updated.labels,
                updated.updated_at,
                updated.id,
            ],
        )?;

        Ok(updated)
    }

    pub fn delete_issue(&self, id: &str) -> SqlResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows = conn.execute("DELETE FROM issues WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn import_issues(&self, issues: Vec<CreateIssueRequest>) -> SqlResult<Vec<Issue>> {
        let mut created = Vec::new();
        for req in issues {
            let issue = self.create_issue(req)?;
            created.push(issue);
        }
        Ok(created)
    }
}

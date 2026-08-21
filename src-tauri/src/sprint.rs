use chrono::Utc;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprint {
    pub id: String,
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    pub status: String,
    pub goal: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSprintRequest {
    pub name: String,
    pub start_date: String,
    pub end_date: String,
    #[serde(default)]
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintItem {
    pub id: String,
    pub sprint_id: String,
    pub issue_id: String,
    pub status: String,
    pub points: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddItemRequest {
    pub sprint_id: String,
    pub issue_id: String,
    #[serde(default = "default_points")]
    pub points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateItemStatusRequest {
    pub item_id: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityData {
    pub sprint_id: String,
    pub sprint_name: String,
    pub total_points: i32,
    pub completed_points: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurndownPoint {
    pub date: String,
    pub remaining: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurndownData {
    pub total_points: i32,
    pub points: Vec<BurndownPoint>,
}

fn default_points() -> i32 {
    1
}

// ── Database Operations ───────────────────────────────────────────────────

pub fn init_sprints_table(conn: &Connection) -> SqlResult<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS sprints (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            start_date  TEXT NOT NULL,
            end_date    TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'planning',
            goal        TEXT NOT NULL DEFAULT '',
            created_at  TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_sprints_status ON sprints(status);

        CREATE TABLE IF NOT EXISTS sprint_items (
            id          TEXT PRIMARY KEY,
            sprint_id   TEXT NOT NULL,
            issue_id    TEXT NOT NULL,
            status      TEXT NOT NULL DEFAULT 'todo',
            points      INTEGER NOT NULL DEFAULT 1,
            created_at  TEXT NOT NULL,
            FOREIGN KEY (sprint_id) REFERENCES sprints(id)
        );

        CREATE INDEX IF NOT EXISTS idx_sprint_items_sprint ON sprint_items(sprint_id);
        ",
    )?;
    Ok(())
}

pub fn create_sprint(conn: &Connection, req: CreateSprintRequest) -> SqlResult<Sprint> {
    let now = Utc::now().to_rfc3339();
    let sprint = Sprint {
        id: Uuid::new_v4().to_string(),
        name: req.name,
        start_date: req.start_date,
        end_date: req.end_date,
        status: "planning".to_string(),
        goal: req.goal,
        created_at: now.clone(),
    };
    conn.execute(
        "INSERT INTO sprints (id, name, start_date, end_date, status, goal, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            sprint.id,
            sprint.name,
            sprint.start_date,
            sprint.end_date,
            sprint.status,
            sprint.goal,
            sprint.created_at,
        ],
    )?;
    Ok(sprint)
}

pub fn get_active_sprint(conn: &Connection) -> SqlResult<Option<Sprint>> {
    let result = conn.query_row(
        "SELECT id, name, start_date, end_date, status, goal, created_at
         FROM sprints WHERE status = 'active' LIMIT 1",
        [],
        |row| {
            Ok(Sprint {
                id: row.get(0)?,
                name: row.get(1)?,
                start_date: row.get(2)?,
                end_date: row.get(3)?,
                status: row.get(4)?,
                goal: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    );
    match result {
        Ok(sprint) => Ok(Some(sprint)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

pub fn list_sprints(conn: &Connection) -> SqlResult<Vec<Sprint>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, start_date, end_date, status, goal, created_at
         FROM sprints ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Sprint {
            id: row.get(0)?,
            name: row.get(1)?,
            start_date: row.get(2)?,
            end_date: row.get(3)?,
            status: row.get(4)?,
            goal: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    rows.collect()
}

pub fn activate_sprint(conn: &Connection, sprint_id: &str) -> SqlResult<()> {
    // Close any currently active sprint first
    conn.execute(
        "UPDATE sprints SET status = 'completed' WHERE status = 'active'",
        [],
    )?;
    // Activate the new one
    conn.execute(
        "UPDATE sprints SET status = 'active' WHERE id = ?1",
        params![sprint_id],
    )?;
    Ok(())
}

pub fn close_sprint(conn: &Connection, sprint_id: &str) -> SqlResult<()> {
    conn.execute(
        "UPDATE sprints SET status = 'completed' WHERE id = ?1",
        params![sprint_id],
    )?;
    Ok(())
}

pub fn add_item_to_sprint(conn: &Connection, req: AddItemRequest) -> SqlResult<SprintItem> {
    let now = Utc::now().to_rfc3339();
    let item = SprintItem {
        id: Uuid::new_v4().to_string(),
        sprint_id: req.sprint_id,
        issue_id: req.issue_id,
        status: "todo".to_string(),
        points: req.points,
        created_at: now.clone(),
    };
    conn.execute(
        "INSERT INTO sprint_items (id, sprint_id, issue_id, status, points, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            item.id,
            item.sprint_id,
            item.issue_id,
            item.status,
            item.points,
            item.created_at,
        ],
    )?;
    Ok(item)
}

pub fn remove_item_from_sprint(conn: &Connection, item_id: &str) -> SqlResult<bool> {
    let rows = conn.execute(
        "DELETE FROM sprint_items WHERE id = ?1",
        params![item_id],
    )?;
    Ok(rows > 0)
}

pub fn update_item_status(conn: &Connection, req: UpdateItemStatusRequest) -> SqlResult<SprintItem> {
    conn.execute(
        "UPDATE sprint_items SET status = ?1 WHERE id = ?2",
        params![req.status, req.item_id],
    )?;
    // Return the updated item
    conn.query_row(
        "SELECT id, sprint_id, issue_id, status, points, created_at
         FROM sprint_items WHERE id = ?1",
        params![req.item_id],
        |row| {
            Ok(SprintItem {
                id: row.get(0)?,
                sprint_id: row.get(1)?,
                issue_id: row.get(2)?,
                status: row.get(3)?,
                points: row.get(4)?,
                created_at: row.get(5)?,
            })
        },
    )
}

pub fn get_sprint_items(conn: &Connection, sprint_id: &str) -> SqlResult<Vec<SprintItem>> {
    let mut stmt = conn.prepare(
        "SELECT id, sprint_id, issue_id, status, points, created_at
         FROM sprint_items WHERE sprint_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![sprint_id], |row| {
        Ok(SprintItem {
            id: row.get(0)?,
            sprint_id: row.get(1)?,
            issue_id: row.get(2)?,
            status: row.get(3)?,
            points: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;
    rows.collect()
}

pub fn calculate_velocity(conn: &Connection, num_sprints: i32) -> SqlResult<Vec<VelocityData>> {
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name,
                COALESCE(SUM(si.points), 0) as total_points,
                COALESCE(SUM(CASE WHEN si.status = 'done' THEN si.points ELSE 0 END), 0) as completed_points
         FROM sprints s
         LEFT JOIN sprint_items si ON si.sprint_id = s.id
         WHERE s.status = 'completed'
         GROUP BY s.id
         ORDER BY s.created_at DESC
         LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![num_sprints], |row| {
        Ok(VelocityData {
            sprint_id: row.get(0)?,
            sprint_name: row.get(1)?,
            total_points: row.get(2)?,
            completed_points: row.get(3)?,
        })
    })?;
    let mut result: Vec<VelocityData> = rows.collect::<SqlResult<Vec<_>>>()?;
    result.reverse(); // chronological order
    Ok(result)
}

pub fn get_sprint_burndown(conn: &Connection, sprint_id: &str) -> SqlResult<BurndownData> {
    // Get total points for the sprint
    let total_points: i32 = conn.query_row(
        "SELECT COALESCE(SUM(points), 0) FROM sprint_items WHERE sprint_id = ?1",
        params![sprint_id],
        |row| row.get(0),
    )?;

    // Get sprint dates
    let (start_date_str, end_date_str): (String, String) = conn.query_row(
        "SELECT start_date, end_date FROM sprints WHERE id = ?1",
        params![sprint_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let start = chrono::NaiveDate::parse_from_str(
        &start_date_str[..10],
        "%Y-%m-%d",
    )
    .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid start_date format".to_string()))?;

    let end = chrono::NaiveDate::parse_from_str(
        &end_date_str[..10],
        "%Y-%m-%d",
    )
    .map_err(|_| rusqlite::Error::InvalidParameterName("Invalid end_date format".to_string()))?;

    // Get all items with their statuses
    let items = get_sprint_items(conn, sprint_id)?;

    let mut points = Vec::new();
    let mut current = start;

    while current <= end {
        let date_str = current.format("%Y-%m-%d").to_string();

        // Count done items: for burndown we track how many points are "completed"
        // We use a simple heuristic: items with status 'done' are subtracted from total
        let mut done_points = 0;
        for item in &items {
            if item.status == "done" {
                done_points += item.points;
            }
        }

        let remaining = total_points - done_points;
        points.push(BurndownPoint {
            date: date_str,
            remaining,
        });
        current = current + chrono::Duration::days(1);
    }

    Ok(BurndownData { total_points, points })
}

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::stats::Stats;
use crate::model::task::{Status, Task, TaskId};
use crate::store::repository::TaskRepository;

/// Shared database connection with reference-counted transaction depth.
pub struct DbConnection {
    conn: Connection,
    tx_depth: RefCell<usize>,
}

pub struct SqliteStore {
    db: Rc<DbConnection>,
    table_name: String,
}

const ALLOWED_TABLES: &[&str] = &["tasks", "archive"];

impl SqliteStore {
    /// Open a database connection with WAL mode.
    pub fn open(db_path: &Path) -> Result<Rc<DbConnection>, AppError> {
        let conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(Rc::new(DbConnection {
            conn,
            tx_depth: RefCell::new(0),
        }))
    }

    /// Create a new store for the given table using a shared connection.
    /// `table_name` must be one of the allowed table names ("tasks", "archive").
    pub fn new(db: Rc<DbConnection>, table_name: &str) -> Result<Self, AppError> {
        if !ALLOWED_TABLES.contains(&table_name) {
            return Err(AppError::InvalidInput(format!(
                "Invalid table name: '{table_name}'"
            )));
        }
        let store = Self {
            db,
            table_name: table_name.to_string(),
        };
        store.ensure_table()?;
        Ok(store)
    }

    /// Begin a transaction (nested calls increment depth without extra BEGIN).
    pub fn set_batch_mode(&self, enabled: bool) {
        if enabled {
            let mut depth = self.db.tx_depth.borrow_mut();
            if *depth == 0 {
                let _ = self.db.conn.execute_batch("BEGIN");
            }
            *depth += 1;
        }
    }

    /// Commit the transaction when the outermost batch ends.
    pub fn flush(&self) -> Result<(), AppError> {
        let mut depth = self.db.tx_depth.borrow_mut();
        if *depth > 0 {
            *depth -= 1;
            if *depth == 0 {
                drop(depth);
                self.db.conn.execute_batch("COMMIT")?;
            }
        }
        Ok(())
    }

    fn ensure_table(&self) -> Result<(), AppError> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                content TEXT,
                status TEXT NOT NULL DEFAULT 'todo',
                priority TEXT NOT NULL DEFAULT 'none',
                created_by TEXT NOT NULL DEFAULT 'human',
                label TEXT,
                project TEXT,
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            )",
            self.table_name
        );
        self.db.conn.execute_batch(&sql)?;
        Ok(())
    }

    fn row_to_task(row: &rusqlite::Row) -> rusqlite::Result<Task> {
        let id_str: String = row.get("id")?;
        let status_str: String = row.get("status")?;
        let priority_str: String = row.get("priority")?;
        let created_by_str: String = row.get("created_by")?;
        let created_at_str: String = row.get("created_at")?;
        let updated_at_str: String = row.get("updated_at")?;
        let completed_at_str: Option<String> = row.get("completed_at")?;
        let parent_id_str: Option<String> = row.get("parent_id")?;

        let parse_err = |e: String| {
            rusqlite::Error::FromSqlConversionFailure(
                0,
                rusqlite::types::Type::Text,
                Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        };

        Ok(Task {
            id: Uuid::parse_str(&id_str).map_err(|e| parse_err(e.to_string()))?,
            title: row.get("title")?,
            content: row.get("content")?,
            status: status_str.parse().map_err(parse_err)?,
            priority: priority_str.parse().map_err(parse_err)?,
            created_by: created_by_str.parse().map_err(parse_err)?,
            label: row.get("label")?,
            project: row.get("project")?,
            parent_id: parent_id_str
                .map(|s| Uuid::parse_str(&s))
                .transpose()
                .map_err(|e| parse_err(e.to_string()))?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| parse_err(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| parse_err(e.to_string()))?
                .with_timezone(&Utc),
            completed_at: completed_at_str
                .map(|s| DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| parse_err(e.to_string()))?
                .map(|dt| dt.with_timezone(&Utc)),
        })
    }
}

impl TaskRepository for SqliteStore {
    fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        let mut conditions = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(ref status) = filter.status {
            conditions.push("status = ?");
            param_values.push(status.to_string());
        }

        if let Some(ref priority) = filter.priority {
            conditions.push("priority = ?");
            param_values.push(priority.to_string());
        }

        if let Some(ref created_by) = filter.created_by {
            conditions.push("created_by = ?");
            param_values.push(created_by.to_string());
        }

        if let Some(ref label) = filter.label {
            conditions.push("label = ?");
            param_values.push(label.clone());
        }

        if let Some(ref project) = filter.project {
            conditions.push("project = ?");
            param_values.push(project.clone());
        }

        if let Some(ref parent_filter) = filter.parent_id {
            match parent_filter {
                Some(id) => {
                    conditions.push("parent_id = ?");
                    param_values.push(id.to_string());
                }
                None => {
                    conditions.push("parent_id IS NULL");
                }
            }
        }

        if !filter.include_done {
            conditions.push("status != 'done'");
        }

        if !filter.include_cancelled {
            conditions.push("status != 'cancelled'");
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT * FROM {} {}", self.table_name, where_clause);
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();

        let mut stmt = self.db.conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(params.as_slice(), Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<Task>>>()?;

        Ok(tasks)
    }

    fn get(&self, id: TaskId) -> Result<Option<Task>, AppError> {
        let sql = format!("SELECT * FROM {} WHERE id = ?1", self.table_name);
        let mut stmt = self.db.conn.prepare(&sql)?;
        let mut rows = stmt.query_map(params![id.to_string()], Self::row_to_task)?;
        match rows.next() {
            Some(row) => Ok(Some(row?)),
            None => Ok(None),
        }
    }

    fn get_by_prefix(&self, prefix: &str) -> Result<Vec<Task>, AppError> {
        let sql = format!("SELECT * FROM {} WHERE id LIKE ?1", self.table_name);
        let pattern = format!("{}%", prefix.to_lowercase());
        let mut stmt = self.db.conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(params![pattern], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<Task>>>()?;
        Ok(tasks)
    }

    fn create(&self, task: Task) -> Result<Task, AppError> {
        let sql = format!(
            "INSERT INTO {} (id, title, content, status, priority, created_by, \
             label, project, parent_id, created_at, updated_at, completed_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            self.table_name
        );
        self.db.conn.execute(
            &sql,
            params![
                task.id.to_string(),
                task.title,
                task.content,
                task.status.to_string(),
                task.priority.to_string(),
                task.created_by.to_string(),
                task.label,
                task.project,
                task.parent_id.map(|id| id.to_string()),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.completed_at.map(|dt| dt.to_rfc3339()),
            ],
        )?;
        Ok(task)
    }

    fn update(&self, task: Task) -> Result<Task, AppError> {
        let sql = format!(
            "UPDATE {} SET title = ?1, content = ?2, status = ?3, priority = ?4, \
             created_by = ?5, label = ?6, project = ?7, parent_id = ?8, \
             created_at = ?9, updated_at = ?10, completed_at = ?11 \
             WHERE id = ?12",
            self.table_name
        );
        let rows = self.db.conn.execute(
            &sql,
            params![
                task.title,
                task.content,
                task.status.to_string(),
                task.priority.to_string(),
                task.created_by.to_string(),
                task.label,
                task.project,
                task.parent_id.map(|id| id.to_string()),
                task.created_at.to_rfc3339(),
                task.updated_at.to_rfc3339(),
                task.completed_at.map(|dt| dt.to_rfc3339()),
                task.id.to_string(),
            ],
        )?;
        if rows == 0 {
            return Err(AppError::TaskNotFound(task.id.to_string()));
        }
        Ok(task)
    }

    fn delete(&self, id: TaskId) -> Result<Option<Task>, AppError> {
        let task = self.get(id)?;
        if task.is_some() {
            let sql = format!("DELETE FROM {} WHERE id = ?1", self.table_name);
            self.db.conn.execute(&sql, params![id.to_string()])?;
        }
        Ok(task)
    }

    fn get_children(&self, parent_id: TaskId) -> Result<Vec<Task>, AppError> {
        let sql = format!("SELECT * FROM {} WHERE parent_id = ?1", self.table_name);
        let mut stmt = self.db.conn.prepare(&sql)?;
        let tasks = stmt
            .query_map(params![parent_id.to_string()], Self::row_to_task)?
            .collect::<rusqlite::Result<Vec<Task>>>()?;
        Ok(tasks)
    }

    fn stats(&self, filter: &TaskFilter) -> Result<Stats, AppError> {
        let tasks = self.list(filter)?;

        let mut by_status = std::collections::HashMap::new();
        let mut by_priority = std::collections::HashMap::new();
        let mut by_label = std::collections::HashMap::new();
        let mut by_project = std::collections::HashMap::new();
        let mut by_creator = std::collections::HashMap::new();

        let mut todo = 0;
        let mut in_progress = 0;
        let mut done = 0;
        let mut cancelled = 0;

        for task in &tasks {
            match task.status {
                Status::Todo => todo += 1,
                Status::InProgress => in_progress += 1,
                Status::Done => done += 1,
                Status::Cancelled => cancelled += 1,
            }

            *by_status.entry(task.status.to_string()).or_insert(0) += 1;
            *by_priority.entry(task.priority.to_string()).or_insert(0) += 1;

            if let Some(ref label) = task.label {
                *by_label.entry(label.clone()).or_insert(0) += 1;
            }

            if let Some(ref project) = task.project {
                *by_project.entry(project.clone()).or_insert(0) += 1;
            }

            *by_creator.entry(task.created_by.to_string()).or_insert(0) += 1;
        }

        Ok(Stats {
            total: tasks.len(),
            todo,
            in_progress,
            done,
            cancelled,
            by_status,
            by_priority,
            by_label,
            by_project,
            by_creator,
        })
    }
}

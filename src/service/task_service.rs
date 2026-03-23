use chrono::Utc;
use serde::Serialize;
use uuid::Uuid;

use crate::config::settings::Settings;
use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::stats::Stats;
use crate::model::task::{CreatedBy, Priority, Status, Task, TaskId};
use crate::service::sanitize;
use crate::store::repository::TaskRepository;
use crate::store::sqlite_store::SqliteStore;

#[derive(Debug, Serialize)]
pub struct StatusChangeResult {
    pub task: Task,
    pub archived: bool,
    pub archived_subtasks: usize,
}

#[derive(Debug, Serialize)]
pub struct DeleteResult {
    pub task: Task,
    pub deleted_subtasks: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchActionResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BatchSummary {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Debug, Serialize)]
pub struct BatchResult {
    pub summary: BatchSummary,
    pub results: Vec<BatchActionResult>,
}

pub struct TaskService {
    store: SqliteStore,
    settings: Settings,
    archive_store: SqliteStore,
}

impl TaskService {
    pub fn new(store: SqliteStore, settings: Settings, archive_store: SqliteStore) -> Self {
        Self {
            store,
            settings,
            archive_store,
        }
    }

    /// Add a new task with validation.
    #[allow(clippy::too_many_arguments)]
    pub fn add_task(
        &self,
        title: String,
        content: Option<String>,
        priority: Priority,
        created_by: CreatedBy,
        label: Option<String>,
        project: Option<String>,
        parent_id: Option<String>,
    ) -> Result<Task, AppError> {
        // Sanitize inputs
        let title = sanitize::sanitize_title(&title, &self.settings)?;
        let content = match content {
            Some(ref c) => sanitize::sanitize_content(c, &self.settings)?,
            None => None,
        };
        let project = match project {
            Some(ref p) => sanitize::sanitize_project(p, &self.settings)?,
            None => None,
        };

        // Validate label
        if let Some(ref l) = label {
            let allowed = self.settings.all_labels();
            if !allowed.contains(l) {
                return Err(AppError::InvalidLabel(l.clone()));
            }
        }

        // Handle parent task
        let resolved_parent_id: Option<TaskId>;
        let mut effective_project = project.clone();

        if let Some(ref parent_prefix) = parent_id {
            // Resolve parent by prefix
            let parent = self.get_task(parent_prefix)?;

            // Check 2-level nesting limit: parent must not be a subtask itself
            if parent.parent_id.is_some() {
                return Err(AppError::NestingTooDeep);
            }

            // Inherit project from parent if not specified
            if effective_project.is_none() {
                effective_project = parent.project.clone();
            }

            resolved_parent_id = Some(parent.id);
        } else {
            resolved_parent_id = None;
        }

        let now = Utc::now();
        let task = Task {
            id: Uuid::new_v4(),
            title,
            content,
            status: Status::Todo,
            priority,
            created_by,
            label,
            project: effective_project,
            parent_id: resolved_parent_id,
            created_at: now,
            updated_at: now,
            completed_at: None,
        };

        self.store.create(task)
    }

    /// Get a task by ID prefix (minimum 4 characters).
    pub fn get_task(&self, prefix: &str) -> Result<Task, AppError> {
        if prefix.len() < 4 {
            return Err(AppError::IdPrefixTooShort(prefix.to_string()));
        }

        let matches = self.store.get_by_prefix(prefix)?;

        match matches.len() {
            0 => Err(AppError::TaskNotFound(prefix.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            count => Err(AppError::AmbiguousId {
                prefix: prefix.to_string(),
                count,
            }),
        }
    }

    /// Get subtasks of a parent task.
    pub fn get_subtasks(&self, parent_id: TaskId) -> Result<Vec<Task>, AppError> {
        self.store.get_children(parent_id)
    }

    /// List tasks with filter applied.
    pub fn list_tasks(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        self.store.list(filter)
    }

    /// List tasks in tree order: root tasks first, each followed by its children.
    pub fn list_tasks_tree(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        let tasks = self.store.list(filter)?;

        let mut roots: Vec<&Task> = tasks.iter().filter(|t| t.parent_id.is_none()).collect();
        roots.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        let mut result = Vec::new();
        for root in &roots {
            result.push((*root).clone());
            let mut children: Vec<&Task> = tasks
                .iter()
                .filter(|t| t.parent_id == Some(root.id))
                .collect();
            children.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            for child in children {
                result.push(child.clone());
            }
        }

        // Also add any orphan children (parent not in the filtered set)
        for task in &tasks {
            if let Some(pid) = task.parent_id
                && !roots.iter().any(|r| r.id == pid)
            {
                result.push(task.clone());
            }
        }

        Ok(result)
    }

    /// Search tasks by query string (case-insensitive substring match on title and content).
    pub fn search_tasks(&self, query: &str, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        let all_tasks = self.store.list(filter)?;
        let query_lower = query.to_lowercase();

        let matched: Vec<Task> = all_tasks
            .into_iter()
            .filter(|task| {
                let title_match = task.title.to_lowercase().contains(&query_lower);
                let desc_match = task
                    .content
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&query_lower));
                title_match || desc_match
            })
            .collect();

        Ok(matched)
    }

    /// Get stats with filter applied.
    pub fn stats(&self, filter: &TaskFilter) -> Result<Stats, AppError> {
        self.store.stats(filter)
    }

    /// Edit a task. All parameters except prefix are optional.
    #[allow(clippy::too_many_arguments)]
    pub fn edit_task(
        &self,
        prefix: &str,
        title: Option<String>,
        content: Option<String>,
        priority: Option<Priority>,
        label: Option<String>,
        project: Option<String>,
        parent: Option<String>,
    ) -> Result<Task, AppError> {
        // Check if at least one field is specified
        if title.is_none()
            && content.is_none()
            && priority.is_none()
            && label.is_none()
            && project.is_none()
            && parent.is_none()
        {
            return Err(AppError::NoEditFields);
        }

        let mut task = self.get_task(prefix)?;

        // Sanitize only changed values — preserves existing data that may exceed current limits
        let title = match title {
            Some(ref t) if *t != task.title => Some(sanitize::sanitize_title(t, &self.settings)?),
            _ => None,
        };
        let content = match content {
            Some(ref c) if task.content.as_deref() != Some(c.as_str()) => {
                sanitize::sanitize_content(c, &self.settings)?
            }
            _ => None,
        };
        let project = match project {
            Some(ref p) if task.project.as_deref() != Some(p.as_str()) => {
                sanitize::sanitize_project(p, &self.settings)?
            }
            _ => None,
        };

        // Validate label if provided
        if let Some(ref l) = label {
            let allowed = self.settings.all_labels();
            if !allowed.contains(l) {
                return Err(AppError::InvalidLabel(l.clone()));
            }
        }

        // Handle parent change
        if let Some(ref parent_str) = parent {
            if parent_str == "none" {
                task.parent_id = None;
            } else {
                let parent_task = self.get_task(parent_str)?;
                // Check 2-level nesting limit
                if parent_task.parent_id.is_some() {
                    return Err(AppError::NestingTooDeep);
                }
                task.parent_id = Some(parent_task.id);
            }
        }

        // Update fields that are Some
        if let Some(t) = title {
            task.title = t;
        }
        if let Some(c) = content {
            task.content = Some(c);
        }
        if let Some(p) = priority {
            task.priority = p;
        }
        if let Some(l) = label {
            task.label = Some(l);
        }
        if let Some(p) = project {
            task.project = Some(p);
        }

        task.updated_at = Utc::now();
        self.store.update(task)
    }

    /// Change the status of a task.
    pub fn change_status(
        &self,
        prefix: &str,
        new_status_str: &str,
    ) -> Result<StatusChangeResult, AppError> {
        let new_status: Status = new_status_str.parse().map_err(AppError::InvalidInput)?;

        let is_archiving = new_status == Status::Done || new_status == Status::Cancelled;
        let is_reverting = !is_archiving;

        // For revert: try store first, then archive_store
        let mut task = if is_reverting {
            match self.get_task(prefix) {
                Ok(t) => t,
                Err(_) => self.get_task_from_archive(prefix)?,
            }
        } else {
            self.get_task(prefix)?
        };

        let old_status = task.status;
        let was_archived = old_status == Status::Done || old_status == Status::Cancelled;

        task.status = new_status;
        task.updated_at = Utc::now();

        // Manage completed_at
        if new_status == Status::Done {
            task.completed_at = Some(Utc::now());
        } else if old_status == Status::Done {
            task.completed_at = None;
        }

        let mut archived = false;
        let mut archived_subtasks = 0;

        if is_archiving {
            // Use transaction for atomic archive operation
            self.store.set_batch_mode(true);

            // Archive the task: create in archive, then delete from store
            self.archive_store.create(task.clone())?;
            self.store.delete(task.id)?;

            // Archive subtasks too
            let children = self.store.get_children(task.id)?;
            for child in &children {
                self.archive_store.create(child.clone())?;
                self.store.delete(child.id)?;
            }
            archived_subtasks = children.len();
            archived = true;

            self.store.flush()?;
        } else if was_archived {
            // Use transaction for atomic restore operation
            self.store.set_batch_mode(true);

            // Revert from archive: restore to store, then delete from archive
            task.status = new_status;
            self.store.create(task.clone())?;
            self.archive_store.delete(task.id)?;

            // Restore subtasks too
            let children = self.archive_store.get_children(task.id)?;
            for child in &children {
                self.store.create(child.clone())?;
                self.archive_store.delete(child.id)?;
                archived_subtasks += 1;
            }

            self.store.flush()?;
        } else {
            // Normal status change (not archiving, not reverting from archive)
            self.store.update(task.clone())?;
        }

        Ok(StatusChangeResult {
            task,
            archived,
            archived_subtasks,
        })
    }

    /// Get a task from the archive by ID prefix.
    pub fn get_task_from_archive(&self, prefix: &str) -> Result<Task, AppError> {
        if prefix.len() < 4 {
            return Err(AppError::IdPrefixTooShort(prefix.to_string()));
        }

        let matches = self.archive_store.get_by_prefix(prefix)?;

        match matches.len() {
            0 => Err(AppError::TaskNotFound(prefix.to_string())),
            1 => Ok(matches.into_iter().next().unwrap()),
            count => Err(AppError::AmbiguousId {
                prefix: prefix.to_string(),
                count,
            }),
        }
    }

    /// List archived tasks with filter applied.
    pub fn list_archive(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        self.archive_store.list(filter)
    }

    /// Search archived tasks by query string.
    pub fn search_archive(&self, query: &str, filter: &TaskFilter) -> Result<Vec<Task>, AppError> {
        let all_tasks = self.archive_store.list(filter)?;
        let query_lower = query.to_lowercase();

        let matched: Vec<Task> = all_tasks
            .into_iter()
            .filter(|task| {
                let title_match = task.title.to_lowercase().contains(&query_lower);
                let desc_match = task
                    .content
                    .as_ref()
                    .is_some_and(|d| d.to_lowercase().contains(&query_lower));
                title_match || desc_match
            })
            .collect();

        Ok(matched)
    }

    /// Delete a task and its children.
    pub fn delete_task(&self, prefix: &str) -> Result<DeleteResult, AppError> {
        let task = self.get_task(prefix)?;
        let children = self.store.get_children(task.id)?;
        let deleted_subtasks = children.len();

        // Delete children first
        for child in &children {
            self.store.delete(child.id)?;
        }

        // Delete the task itself
        self.store.delete(task.id)?;

        Ok(DeleteResult {
            task,
            deleted_subtasks,
        })
    }

    /// Execute a batch of actions with a single write at the end.
    pub fn batch(&self, actions: Vec<serde_json::Value>) -> Result<BatchResult, AppError> {
        self.store.set_batch_mode(true);
        self.archive_store.set_batch_mode(true);

        let total = actions.len();
        let mut results = Vec::new();
        let mut succeeded = 0;
        let mut failed = 0;

        for action in &actions {
            let result = self.execute_batch_action(action);
            match result {
                Ok(task) => {
                    succeeded += 1;
                    results.push(BatchActionResult {
                        success: true,
                        task: Some(task),
                        error: None,
                    });
                }
                Err(e) => {
                    failed += 1;
                    results.push(BatchActionResult {
                        success: false,
                        task: None,
                        error: Some(e.to_string()),
                    });
                }
            }
        }

        self.store.set_batch_mode(false);
        self.store.flush()?;
        self.archive_store.set_batch_mode(false);
        self.archive_store.flush()?;

        Ok(BatchResult {
            summary: BatchSummary {
                total,
                succeeded,
                failed,
            },
            results,
        })
    }

    fn execute_batch_action(&self, action: &serde_json::Value) -> Result<Task, AppError> {
        let action_type = action["action"]
            .as_str()
            .ok_or_else(|| AppError::InvalidInput("Missing 'action' field".to_string()))?;

        match action_type {
            "add" => {
                let title = action["title"]
                    .as_str()
                    .ok_or_else(|| AppError::InvalidInput("Missing 'title' for add".to_string()))?
                    .to_string();

                let content = action["content"].as_str().map(|s| s.to_string());

                let priority = match action["priority"].as_str() {
                    Some(p) => match p {
                        "none" => Priority::None,
                        "low" => Priority::Low,
                        "medium" => Priority::Medium,
                        "high" => Priority::High,
                        "critical" => Priority::Critical,
                        other => {
                            return Err(AppError::InvalidInput(format!(
                                "Invalid priority: '{other}'"
                            )));
                        }
                    },
                    None => Priority::None,
                };

                let created_by = match action["created_by"].as_str() {
                    Some("ai") => CreatedBy::Ai,
                    Some("human") | None => CreatedBy::Human,
                    Some(other) => {
                        return Err(AppError::InvalidInput(format!(
                            "Invalid created_by: '{other}'"
                        )));
                    }
                };

                let label = action["label"].as_str().map(|s| s.to_string());
                let project = action["project"].as_str().map(|s| s.to_string());
                let parent_id = action["parent_id"].as_str().map(|s| s.to_string());

                self.add_task(
                    title, content, priority, created_by, label, project, parent_id,
                )
            }
            "status" => {
                let id = action["id"]
                    .as_str()
                    .ok_or_else(|| AppError::InvalidInput("Missing 'id' for status".to_string()))?;
                let status = action["status"].as_str().ok_or_else(|| {
                    AppError::InvalidInput("Missing 'status' for status action".to_string())
                })?;
                let result = self.change_status(id, status)?;
                Ok(result.task)
            }
            "edit" => {
                let id = action["id"]
                    .as_str()
                    .ok_or_else(|| AppError::InvalidInput("Missing 'id' for edit".to_string()))?;

                let title = action["title"].as_str().map(|s| s.to_string());
                let content = action["content"].as_str().map(|s| s.to_string());
                let priority = action["priority"].as_str().map(|p| match p {
                    "none" => Priority::None,
                    "low" => Priority::Low,
                    "medium" => Priority::Medium,
                    "high" => Priority::High,
                    "critical" => Priority::Critical,
                    _ => Priority::None,
                });
                let label = action["label"].as_str().map(|s| s.to_string());
                let project = action["project"].as_str().map(|s| s.to_string());
                let parent = action["parent"].as_str().map(|s| s.to_string());

                self.edit_task(id, title, content, priority, label, project, parent)
            }
            "delete" => {
                let id = action["id"]
                    .as_str()
                    .ok_or_else(|| AppError::InvalidInput("Missing 'id' for delete".to_string()))?;
                let result = self.delete_task(id)?;
                Ok(result.task)
            }
            other => Err(AppError::InvalidInput(format!(
                "Unknown batch action: '{other}'"
            ))),
        }
    }
}

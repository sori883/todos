use super::task::{CreatedBy, Priority, Status, TaskId};

#[derive(Debug, Default)]
pub struct TaskFilter {
    pub status: Option<Status>,
    pub priority: Option<Priority>,
    pub created_by: Option<CreatedBy>,
    pub label: Option<String>,
    pub project: Option<String>,
    pub parent_id: Option<Option<TaskId>>,
    pub include_done: bool,
    pub include_cancelled: bool,
}

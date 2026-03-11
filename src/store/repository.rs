use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::stats::Stats;
use crate::model::task::{Task, TaskId};

pub trait TaskRepository {
    fn list(&self, filter: &TaskFilter) -> Result<Vec<Task>, AppError>;
    fn get(&self, id: TaskId) -> Result<Option<Task>, AppError>;
    fn get_by_prefix(&self, prefix: &str) -> Result<Vec<Task>, AppError>;
    fn create(&self, task: Task) -> Result<Task, AppError>;
    fn update(&self, task: Task) -> Result<Task, AppError>;
    fn delete(&self, id: TaskId) -> Result<Option<Task>, AppError>;
    fn get_children(&self, parent_id: TaskId) -> Result<Vec<Task>, AppError>;
    fn stats(&self, filter: &TaskFilter) -> Result<Stats, AppError>;
}

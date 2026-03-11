use serde::{Deserialize, Serialize};

use crate::model::task::Task;

pub const CURRENT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskData {
    pub version: u32,
    pub tasks: Vec<Task>,
}

impl TaskData {
    pub fn empty() -> Self {
        Self {
            version: CURRENT_VERSION,
            tasks: Vec::new(),
        }
    }
}

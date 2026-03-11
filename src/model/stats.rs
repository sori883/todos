use std::collections::HashMap;

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Stats {
    pub total: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub done: usize,
    pub cancelled: usize,
    pub by_status: HashMap<String, usize>,
    pub by_priority: HashMap<String, usize>,
    pub by_label: HashMap<String, usize>,
    pub by_project: HashMap<String, usize>,
    pub by_creator: HashMap<String, usize>,
}

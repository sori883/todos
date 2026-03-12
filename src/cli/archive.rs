use serde::Serialize;

use crate::cli::output::{CliResponse, print_response};
use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::task::{Priority, Task};
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct ArchiveData {
    count: usize,
    tasks: Vec<Task>,
}

pub struct ArchiveParams {
    pub filter: TaskFilter,
    pub query: Option<String>,
    pub sort: String,
    pub reverse: bool,
    pub limit: Option<usize>,
}

pub fn run(service: &TaskService, params: ArchiveParams, format: &str) -> Result<(), AppError> {
    let mut tasks = if let Some(ref q) = params.query {
        service.search_archive(q, &params.filter)?
    } else {
        service.list_archive(&params.filter)?
    };

    // Sort tasks
    sort_tasks(&mut tasks, &params.sort, params.reverse);

    // Apply limit
    if let Some(limit) = params.limit {
        tasks.truncate(limit);
    }

    let count = tasks.len();
    let data = ArchiveData {
        count,
        tasks: tasks.clone(),
    };

    if format == "json" {
        let response = CliResponse::success(data);
        print_response(&response, format);
    } else {
        let msg = format_text_table(&tasks);
        let response = CliResponse::success_with_message(data, msg);
        print_response(&response, format);
    }

    Ok(())
}

fn priority_sort_value(p: &Priority) -> u8 {
    match p {
        Priority::None => 0,
        Priority::Low => 1,
        Priority::Medium => 2,
        Priority::High => 3,
        Priority::Critical => 4,
    }
}

fn sort_tasks(tasks: &mut [Task], sort_field: &str, reverse: bool) {
    tasks.sort_by(|a, b| {
        let cmp = match sort_field {
            "priority" => priority_sort_value(&b.priority).cmp(&priority_sort_value(&a.priority)),
            "updated_at" => a.updated_at.cmp(&b.updated_at),
            "title" => a.title.cmp(&b.title),
            _ => a.created_at.cmp(&b.created_at),
        };
        if reverse { cmp.reverse() } else { cmp }
    });
}

fn format_text_table(tasks: &[Task]) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{:<10} {:<14} {:<10} {:<30} {:<16} {}",
        "ID", "Status", "Priority", "Title", "Project", "Label"
    ));

    for task in tasks {
        let id_short = &task.id.to_string()[..8];
        let project = task.project.as_deref().unwrap_or("");
        let label = task.label.as_deref().unwrap_or("");
        let indent = if task.parent_id.is_some() { "  " } else { "" };
        let title = format!("{}{}", indent, task.title);
        lines.push(format!(
            "{:<10} {:<14} {:<10} {:<30} {:<16} {}",
            id_short, task.status, task.priority, title, project, label
        ));
    }

    lines.join("\n")
}

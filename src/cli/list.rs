use serde::Serialize;

use crate::cli::output::{print_response, CliResponse};
use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::model::task::{Priority, Task};
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct ListData {
    count: usize,
    tasks: Vec<Task>,
}

pub struct ListParams {
    pub filter: TaskFilter,
    pub sort: String,
    pub reverse: bool,
    pub limit: Option<usize>,
    pub flat: bool,
}

pub fn run(service: &TaskService, params: ListParams, format: &str) -> Result<(), AppError> {
    let mut tasks = if params.flat {
        service.list_tasks(&params.filter)?
    } else {
        service.list_tasks_tree(&params.filter)?
    };

    // Sort tasks
    if params.flat {
        sort_tasks(&mut tasks, &params.sort, params.reverse);
    } else {
        sort_tree_tasks(&mut tasks, &params.sort, params.reverse);
    }

    // Apply limit
    if let Some(limit) = params.limit {
        tasks.truncate(limit);
    }

    let count = tasks.len();
    let data = ListData {
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
            _ => a.created_at.cmp(&b.created_at), // default: created_at
        };
        if reverse { cmp.reverse() } else { cmp }
    });
}

fn sort_tree_tasks(tasks: &mut Vec<Task>, sort_field: &str, reverse: bool) {
    // Separate roots and children
    let mut roots: Vec<Task> = Vec::new();
    let mut children: Vec<Task> = Vec::new();

    for task in tasks.drain(..) {
        if task.parent_id.is_none() {
            roots.push(task);
        } else {
            children.push(task);
        }
    }

    // Sort roots
    sort_tasks(&mut roots, sort_field, reverse);

    // Rebuild tree order
    for root in &roots {
        tasks.push(root.clone());
        let mut root_children: Vec<Task> = children
            .iter()
            .filter(|c| c.parent_id == Some(root.id))
            .cloned()
            .collect();
        sort_tasks(&mut root_children, sort_field, reverse);
        tasks.extend(root_children);
    }

    // Add orphan children
    for child in &children {
        if let Some(pid) = child.parent_id {
            if !roots.iter().any(|r| r.id == pid) {
                tasks.push(child.clone());
            }
        }
    }
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

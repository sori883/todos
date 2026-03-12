use serde::Serialize;

use crate::cli::output::{CliResponse, print_response};
use crate::error::AppError;
use crate::model::task::Task;
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct ShowData {
    task: Task,
    #[serde(skip_serializing_if = "Option::is_none")]
    subtasks: Option<Vec<Task>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<ParentInfo>,
}

#[derive(Debug, Serialize)]
struct ParentInfo {
    id: String,
    title: String,
}

pub fn run(service: &TaskService, id_prefix: &str, format: &str) -> Result<(), AppError> {
    let task = match service.get_task(id_prefix) {
        Ok(t) => t,
        Err(_) => service.get_task_from_archive(id_prefix)?,
    };

    // If this is a parent task (no parent_id), get subtasks
    let subtasks = if task.parent_id.is_none() {
        let children = service.get_subtasks(task.id)?;
        Some(children)
    } else {
        None
    };

    // If this is a subtask, get parent info
    let parent = if let Some(parent_id) = task.parent_id {
        // Get parent task directly by its full ID (use first 8 chars as prefix)
        let parent_prefix = parent_id.to_string();
        match service.get_task(&parent_prefix[..8]) {
            Ok(parent_task) => Some(ParentInfo {
                id: parent_task.id.to_string(),
                title: parent_task.title.clone(),
            }),
            Err(_) => None,
        }
    } else {
        None
    };

    let data = ShowData {
        task: task.clone(),
        subtasks,
        parent,
    };

    let response = if format == "text" {
        // For text output, build a readable display
        let msg = format_text_output(&data);
        CliResponse::success_with_message(data, msg)
    } else {
        CliResponse::success(data)
    };

    print_response(&response, format);

    Ok(())
}

fn format_text_output(data: &ShowData) -> String {
    let task = &data.task;
    let mut lines = Vec::new();

    lines.push(format!("ID:          {}", task.id));
    lines.push(format!("Title:       {}", task.title));
    if let Some(ref content) = task.content {
        lines.push(format!("Content: {content}"));
    }
    lines.push(format!("Status:      {}", task.status));
    lines.push(format!("Priority:    {}", task.priority));
    lines.push(format!("Created by:  {}", task.created_by));
    if let Some(ref label) = task.label {
        lines.push(format!("Label:       {label}"));
    }
    if let Some(ref project) = task.project {
        lines.push(format!("Project:     {project}"));
    }
    lines.push(format!("Created at:  {}", task.created_at));
    lines.push(format!("Updated at:  {}", task.updated_at));

    if let Some(ref parent) = data.parent {
        lines.push(format!(
            "Parent:      {}.. {}",
            &parent.id[..8],
            parent.title
        ));
    }

    if let Some(ref subtasks) = data.subtasks {
        lines.push(String::new());
        lines.push(format!("Subtasks ({}):", subtasks.len()));
        for sub in subtasks {
            lines.push(format!(
                "  {}.. {} {} {}",
                &sub.id.to_string()[..8],
                sub.status,
                sub.priority,
                sub.title
            ));
        }
    }

    lines.join("\n")
}

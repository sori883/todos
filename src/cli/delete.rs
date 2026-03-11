use serde::Serialize;
use std::io::{self, BufRead, Write};

use crate::cli::output::{print_response, CliResponse};
use crate::error::AppError;
use crate::model::task::Task;
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct DeleteData {
    task: Task,
    deleted_subtasks: usize,
}

pub fn run(
    service: &TaskService,
    id: &str,
    yes: bool,
    format: &str,
) -> Result<(), AppError> {
    // For JSON format, require --yes
    if format == "json" && !yes {
        return Err(AppError::InvalidInput(
            "JSON format requires --yes flag for delete".to_string(),
        ));
    }

    // If not --yes, prompt for confirmation
    if !yes {
        let task = service.get_task(id)?;
        let children = service.get_subtasks(task.id)?;

        if children.is_empty() {
            print!("Delete '{}' ? [y/N] ", task.title);
        } else {
            print!(
                "Delete '{}' and {} subtask(s)? [y/N] ",
                task.title,
                children.len()
            );
        }
        io::stdout().flush().map_err(|e| AppError::Io(e))?;

        let stdin = io::stdin();
        let line = stdin
            .lock()
            .lines()
            .next()
            .unwrap_or(Ok(String::new()))
            .map_err(|e| AppError::Io(e))?;

        if !line.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled");
            return Ok(());
        }
    }

    let result = service.delete_task(id)?;

    let data = DeleteData {
        task: result.task.clone(),
        deleted_subtasks: result.deleted_subtasks,
    };

    if format == "text" {
        let mut msg = format!("Deleted '{}'", result.task.title);
        if result.deleted_subtasks > 0 {
            msg.push_str(&format!(" and {} subtask(s)", result.deleted_subtasks));
        }
        let response = CliResponse::success_with_message(data, msg);
        print_response(&response, format);
    } else {
        let response = CliResponse::success(data);
        print_response(&response, format);
    }

    Ok(())
}

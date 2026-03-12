use serde::Serialize;

use crate::cli::output::{print_response, CliResponse};
use crate::error::AppError;
use crate::i18n::{get_message, Message};
use crate::model::task::{Priority, Task};
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct EditData {
    task: Task,
}

pub struct EditParams {
    pub id: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub priority: Option<Priority>,
    pub label: Option<String>,
    pub project: Option<String>,
    pub parent: Option<String>,
}

pub fn run(
    service: &TaskService,
    params: EditParams,
    format: &str,
    locale: &str,
) -> Result<(), AppError> {
    let task = service.edit_task(
        &params.id,
        params.title,
        params.content,
        params.priority,
        params.label,
        params.project,
        params.parent,
    )?;

    let data = EditData { task };
    let response = if format == "text" {
        CliResponse::success_with_message(data, get_message(Message::TaskUpdated, locale))
    } else {
        CliResponse::success(data)
    };
    print_response(&response, format);

    Ok(())
}

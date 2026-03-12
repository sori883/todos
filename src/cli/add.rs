use serde::Serialize;

use crate::cli::output::{print_response, CliResponse};
use crate::error::AppError;
use crate::i18n::{get_message, Message};
use crate::model::task::{CreatedBy, Priority, Task};
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct AddData {
    task: Task,
}

pub struct AddParams {
    pub title: String,
    pub content: Option<String>,
    pub priority: Priority,
    pub created_by: CreatedBy,
    pub label: Option<String>,
    pub project: Option<String>,
    pub parent: Option<String>,
}

pub fn run(
    service: &TaskService,
    params: AddParams,
    format: &str,
    locale: &str,
) -> Result<(), AppError> {
    let task = service.add_task(
        params.title,
        params.content,
        params.priority,
        params.created_by,
        params.label,
        params.project,
        params.parent,
    )?;

    let response = CliResponse::success_with_message(
        AddData { task },
        get_message(Message::TaskCreated, locale),
    );
    print_response(&response, format);

    Ok(())
}

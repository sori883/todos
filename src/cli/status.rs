use serde::Serialize;

use crate::cli::output::{CliResponse, print_response};
use crate::error::AppError;
use crate::i18n::{Message, get_message};
use crate::model::task::Task;
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct StatusData {
    task: Task,
}

pub fn run(
    service: &TaskService,
    id: &str,
    status: &str,
    format: &str,
    locale: &str,
) -> Result<(), AppError> {
    let result = service.change_status(id, status)?;

    let data = StatusData {
        task: result.task.clone(),
    };

    if format == "text" {
        let mut msg = get_message(
            Message::StatusChanged(result.task.status.to_string()),
            locale,
        );
        if result.archived {
            msg.push('\n');
            if result.archived_subtasks > 0 {
                msg.push_str(&get_message(
                    Message::TaskArchivedWithSubtasks(result.archived_subtasks),
                    locale,
                ));
            } else {
                msg.push_str(&get_message(Message::TaskArchived, locale));
            }
        }
        let response = CliResponse::success_with_message(data, msg);
        print_response(&response, format);
    } else {
        let response = CliResponse::success(data);
        print_response(&response, format);
    }

    Ok(())
}

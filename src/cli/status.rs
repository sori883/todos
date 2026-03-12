use serde::Serialize;

use crate::cli::output::{print_response, CliResponse};
use crate::error::AppError;
use crate::i18n::{get_message, Message};
use crate::model::task::Task;
use crate::service::task_service::TaskService;

#[derive(Debug, Serialize)]
struct StatusData {
    task: Task,
    generated_task: Option<Task>,
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
        generated_task: result.generated_task.clone(),
    };

    if format == "text" {
        let mut msg = get_message(
            Message::StatusChanged(result.task.status.to_string()),
            locale,
        );
        if let Some(ref generated) = result.generated_task {
            msg.push('\n');
            msg.push_str(&get_message(
                Message::RecurringTaskGenerated(generated.title.clone()),
                locale,
            ));
        }
        let response = CliResponse::success_with_message(data, msg);
        print_response(&response, format);
    } else {
        let response = CliResponse::success(data);
        print_response(&response, format);
    }

    Ok(())
}

use crate::cli::output::{CliResponse, print_response};
use crate::error::AppError;
use crate::model::filter::TaskFilter;
use crate::service::task_service::TaskService;

pub fn run(service: &TaskService, filter: &TaskFilter, format: &str) -> Result<(), AppError> {
    let stats = service.stats(filter)?;

    if format == "json" {
        let response = CliResponse::success(stats);
        print_response(&response, format);
    } else {
        let mut lines = Vec::new();
        lines.push(format!("Total: {}", stats.total));
        lines.push(format!(
            "  todo: {}, in_progress: {}, done: {}, cancelled: {}",
            stats.todo, stats.in_progress, stats.done, stats.cancelled
        ));

        if !stats.by_priority.is_empty() {
            lines.push("Priority:".to_string());
            for (k, v) in &stats.by_priority {
                lines.push(format!("  {k}: {v}"));
            }
        }

        if !stats.by_label.is_empty() {
            lines.push("Labels:".to_string());
            for (k, v) in &stats.by_label {
                lines.push(format!("  {k}: {v}"));
            }
        }

        if !stats.by_project.is_empty() {
            lines.push("Projects:".to_string());
            for (k, v) in &stats.by_project {
                lines.push(format!("  {k}: {v}"));
            }
        }

        if !stats.by_creator.is_empty() {
            lines.push("Creators:".to_string());
            for (k, v) in &stats.by_creator {
                lines.push(format!("  {k}: {v}"));
            }
        }

        let msg = lines.join("\n");
        let response = CliResponse::success_with_message(stats, msg);
        print_response(&response, format);
    }

    Ok(())
}

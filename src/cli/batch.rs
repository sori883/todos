use std::io::{self, Read};

use crate::cli::output::{CliResponse, print_response};
use crate::error::AppError;
use crate::service::task_service::TaskService;

pub fn run(service: &TaskService, format: &str) -> Result<(), AppError> {
    // Read JSON from stdin
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| AppError::InvalidInput(format!("Failed to read stdin: {e}")))?;

    // Parse the JSON array
    let actions: Vec<serde_json::Value> = serde_json::from_str(&input)
        .map_err(|e| AppError::InvalidInput(format!("Invalid JSON: {e}")))?;

    let result = service.batch(actions)?;

    let response = CliResponse::success(result);
    print_response(&response, format);

    Ok(())
}

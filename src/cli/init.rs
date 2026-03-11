use std::fs;
use std::path::Path;

use crate::cli::output::{print_error, print_response, CliResponse};
use crate::error::AppError;
use crate::store::schema::TaskData;

pub fn run(data_dir: &Path, force: bool, format: &str) -> Result<(), AppError> {
    let tasks_path = data_dir.join("tasks.json");

    // Check if already initialized
    if tasks_path.exists() && !force {
        let err = AppError::AlreadyInitialized;
        print_error(&err.to_string(), format);
        return Err(err);
    }

    // Create directory
    fs::create_dir_all(data_dir)?;

    // Write empty tasks.json
    let data = TaskData::empty();
    let content = serde_json::to_string_pretty(&data)?;
    fs::write(&tasks_path, content)?;

    let response = CliResponse::<serde_json::Value>::success_with_message(
        serde_json::json!({
            "path": data_dir.to_string_lossy()
        }),
        format!("Initialized todos in {}", data_dir.display()),
    );
    print_response(&response, format);

    Ok(())
}

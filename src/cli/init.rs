use std::fs;
use std::path::Path;

use crate::cli::output::{CliResponse, print_error, print_response};
use crate::config::paths;
use crate::error::AppError;
use crate::i18n::{Message, get_message};
use crate::store::sqlite_store::SqliteStore;

pub fn run(data_dir: &Path, force: bool, format: &str, locale: &str) -> Result<(), AppError> {
    let db_path = paths::db_path(data_dir);

    // Check if already initialized
    if db_path.exists() && !force {
        let err = AppError::AlreadyInitialized;
        print_error(&err.to_string(), format);
        return Err(err);
    }

    // Create directory
    fs::create_dir_all(data_dir)?;

    // If force, remove existing database and WAL/SHM sidecar files
    if force {
        for ext in ["db", "db-wal", "db-shm"] {
            let p = data_dir.join(format!("todos.{ext}"));
            if p.exists() {
                fs::remove_file(&p)?;
            }
        }
    }

    // Create database with tables
    let conn = SqliteStore::open(&db_path)?;
    SqliteStore::new(conn.clone(), "tasks")?;
    SqliteStore::new(conn, "archive")?;

    let path_str = data_dir.to_string_lossy().to_string();
    let response = CliResponse::<serde_json::Value>::success_with_message(
        serde_json::json!({
            "path": path_str
        }),
        get_message(Message::Initialized(path_str), locale),
    );
    print_response(&response, format);

    Ok(())
}

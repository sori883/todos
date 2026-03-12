use std::path::{Path, PathBuf};

const TODOS_DIR: &str = ".todos";

/// Walk up from `start` looking for `.todos/` directory.
/// Stop at the home directory. If not found, return `start/.todos/`.
pub fn find_data_dir(start: &Path) -> PathBuf {
    let home = dirs::home_dir();
    let mut current = start.to_path_buf();

    loop {
        let candidate = current.join(TODOS_DIR);
        if candidate.is_dir() {
            return candidate;
        }

        // Stop at home directory
        if let Some(ref home_dir) = home
            && current == *home_dir
        {
            break;
        }

        // Move to parent
        match current.parent() {
            Some(parent) => {
                // If we've gone past home, stop
                if let Some(ref home_dir) = home
                    && !current.starts_with(home_dir)
                {
                    break;
                }
                current = parent.to_path_buf();
            }
            None => break,
        }
    }

    // Default: create in start directory
    start.join(TODOS_DIR)
}

/// Resolve the data directory based on CLI options.
/// When `--data-dir` is specified, use that path with `.todos/` appended.
pub fn resolve_data_dir(explicit: Option<&Path>) -> PathBuf {
    match explicit {
        Some(path) => path.join(TODOS_DIR),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            find_data_dir(&cwd)
        }
    }
}

/// Return the tasks.json path within the data directory.
pub fn tasks_json_path(data_dir: &Path) -> PathBuf {
    data_dir.join("tasks.json")
}

/// Return the archive.json path within the data directory.
pub fn archive_json_path(data_dir: &Path) -> PathBuf {
    data_dir.join("archive.json")
}

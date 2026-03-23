use std::path::{Path, PathBuf};

const TODOS_DIR: &str = ".todos";

/// Walk up from `start` looking for `.todos/` directory.
/// Stop at the home directory. If not found, fall back to `~/.todos/`.
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

    // Fallback: home directory (auto-init on first write)
    match home {
        Some(home_dir) => home_dir.join(TODOS_DIR),
        None => start.join(TODOS_DIR),
    }
}

/// Resolve the data directory based on CLI options.
/// When `--data-dir` is specified, use that path with `.todos/` appended.
/// Otherwise, walk up from cwd looking for `.todos/`, falling back to `~/.todos/`.
pub fn resolve_data_dir(explicit: Option<&Path>) -> PathBuf {
    match explicit {
        Some(path) => path.join(TODOS_DIR),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            find_data_dir(&cwd)
        }
    }
}

/// Resolve the data directory for the `init` command.
/// Uses explicit `--data-dir` if provided, otherwise current directory.
/// Unlike `resolve_data_dir`, this does NOT walk up the directory tree.
pub fn init_data_dir(explicit: Option<&Path>) -> PathBuf {
    match explicit {
        Some(path) => path.join(TODOS_DIR),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            cwd.join(TODOS_DIR)
        }
    }
}

/// Return the database file path within the data directory.
pub fn db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("todos.db")
}

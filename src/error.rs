use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Task not found: '{0}'")]
    TaskNotFound(String),

    #[error("Ambiguous ID '{prefix}': {count} tasks match")]
    AmbiguousId { prefix: String, count: usize },

    #[error("ID prefix too short: '{0}' (minimum 4 characters)")]
    IdPrefixTooShort(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid label: '{0}'")]
    InvalidLabel(String),

    #[error("No edit fields specified")]
    NoEditFields,

    #[error("Nesting too deep: subtasks cannot have children")]
    NestingTooDeep,

    #[error("Data file error: {0}")]
    DataFile(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Already initialized: use --force to overwrite")]
    AlreadyInitialized,
}

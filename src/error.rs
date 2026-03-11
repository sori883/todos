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

    #[error("Subtasks cannot have recurrence")]
    SubtaskRecurrence,

    #[error("Recurrence generation failed: {0}")]
    RecurrenceGenerationFailed(String),

    #[error("Data file error: {0}")]
    DataFile(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Config error: {0}")]
    Config(String),

    #[error("File lock error: {0}")]
    FileLock(String),

    #[error("Schema version {0} is newer than supported version {1}")]
    SchemaVersionTooNew(u32, u32),

    #[error("Schema migration failed: {0}")]
    SchemaMigration(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Already initialized: use --force to overwrite")]
    AlreadyInitialized,
}

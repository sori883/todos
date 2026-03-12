pub mod en;
pub mod ja;

/// Message keys for i18n
pub enum Message {
    TaskCreated,
    TaskUpdated,
    TaskDeleted,
    TaskDeletedWithSubtasks(usize),
    StatusChanged(String),
    RecurringTaskGenerated(String),
    Initialized(String),
}

/// Get a localized message string.
/// Falls back to "en" for unknown locales.
pub fn get_message(msg: Message, locale: &str) -> String {
    match locale {
        "ja" => ja::get(msg),
        "en" => en::get(msg),
        _ => en::get(msg), // fallback to en
    }
}

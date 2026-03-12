use super::Message;

pub fn get(msg: Message) -> String {
    match msg {
        Message::TaskCreated => "Task created".to_string(),
        Message::TaskUpdated => "Task updated".to_string(),
        Message::TaskDeleted => "Task deleted".to_string(),
        Message::TaskDeletedWithSubtasks(n) => format!("Task deleted with {n} subtask(s)"),
        Message::StatusChanged(s) => format!("Status changed to {s}"),
        Message::RecurringTaskGenerated(title) => format!("Recurring task generated: {title}"),
        Message::Initialized(path) => format!("Initialized todos in {path}"),
    }
}

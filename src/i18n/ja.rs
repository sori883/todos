use super::Message;

pub fn get(msg: Message) -> String {
    match msg {
        Message::TaskCreated => "タスクを作成しました".to_string(),
        Message::TaskUpdated => "タスクを更新しました".to_string(),
        Message::TaskDeleted => "タスクを削除しました".to_string(),
        Message::TaskDeletedWithSubtasks(n) => format!("タスクとサブタスク{n}件を削除しました"),
        Message::StatusChanged(s) => format!("ステータスを{s}に変更しました"),
        Message::Initialized(path) => format!("{path} に初期化しました"),
        Message::TaskArchived => "アーカイブしました".to_string(),
        Message::TaskArchivedWithSubtasks(n) => format!("サブタスク{n}件と共にアーカイブしました"),
        Message::TaskRestored => "復元しました".to_string(),
        Message::TaskRestoredWithSubtasks(n) => format!("サブタスク{n}件と共に復元しました"),
    }
}

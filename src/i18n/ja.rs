use super::Message;

pub fn get(msg: Message) -> String {
    match msg {
        Message::TaskCreated => "タスクを作成しました".to_string(),
        Message::TaskUpdated => "タスクを更新しました".to_string(),
        Message::TaskDeleted => "タスクを削除しました".to_string(),
        Message::TaskDeletedWithSubtasks(n) => format!("タスクとサブタスク{n}件を削除しました"),
        Message::StatusChanged(s) => format!("ステータスを{s}に変更しました"),
        Message::RecurringTaskGenerated(title) => {
            format!("繰り返しタスクを生成しました: {title}")
        }
        Message::Initialized(path) => format!("{path} に初期化しました"),
    }
}

# 06: status コマンド

## 概要

ステータス変更。`completed_at` の自動管理と繰り返しタスクの自動生成を含む。

## 依存タスク

03-add-show

## 先に書く E2E テスト

### 基本ステータス変更

```rust
#[test]
fn status_change_to_done() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert_eq!(result["data"]["task"]["status"], "done");
    assert!(result["data"]["task"]["completed_at"].is_string());
}

#[test]
fn status_change_to_in_progress() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "in_progress"]);
    assert_eq!(result["data"]["task"]["status"], "in_progress");
}

#[test]
fn status_change_to_cancelled() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "cancelled"]);
    assert_eq!(result["data"]["task"]["status"], "cancelled");
}

#[test]
fn status_done_sets_completed_at() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_json(dir.path(), &["status", &id[..8], "done"]);
    let show = todos_json(dir.path(), &["show", &id[..8]]);
    assert!(show["data"]["task"]["completed_at"].is_string());
}

#[test]
fn status_revert_from_done_clears_completed_at() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["status", &id[..8], "done"]).assert().success();
    let result = todos_json(dir.path(), &["status", &id[..8], "todo"]);
    assert!(result["data"]["task"]["completed_at"].is_null());
}
```

### 繰り返しタスク

```rust
#[test]
fn status_done_generates_recurrence_task() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "日次タスク", "--recurrence", "daily"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    // 自動生成されたタスクが返される
    assert!(result["data"]["generated_task"].is_object());
    assert_eq!(result["data"]["generated_task"]["title"], "日次タスク");
    assert_eq!(result["data"]["generated_task"]["status"], "todo");
    assert_eq!(result["data"]["generated_task"]["recurrence"], "daily");
    // 元のタスクと新しいタスクの ID が異なる
    assert_ne!(result["data"]["generated_task"]["id"], id);
}

#[test]
fn status_done_weekly_generates_task() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "週次", "--recurrence", "weekly"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert!(result["data"]["generated_task"].is_object());
}

#[test]
fn status_done_days_of_week_generates_task() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "MWF", "--recurrence", "mon,wed,fri"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert!(result["data"]["generated_task"].is_object());
}

#[test]
fn status_done_no_recurrence_no_generation() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "通常タスク"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert!(result["data"]["generated_task"].is_null());
}

#[test]
fn recurrence_inherits_fields() {
    let dir = setup();
    let t = todos_json(dir.path(), &[
        "add", "定期", "--recurrence", "daily", "-p", "high", "-l", "chore", "-P", "user-service"
    ]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    let gen = &result["data"]["generated_task"];
    assert_eq!(gen["priority"], "high");
    assert_eq!(gen["label"], "chore");
    assert_eq!(gen["project"], "user-service");
    assert_eq!(gen["recurrence"], "daily");
    assert!(gen["completed_at"].is_null());
}

#[test]
fn recurrence_does_not_copy_parent_id() {
    let dir = setup();
    // 親タスクに繰り返しは設定可能（ルートタスク）
    let t = todos_json(dir.path(), &["add", "繰り返し親", "--recurrence", "weekly"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert!(result["data"]["generated_task"]["parent_id"].is_null());
}

#[test]
fn status_cancelled_does_not_generate() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "タスク", "--recurrence", "daily"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "cancelled"]);
    assert!(result["data"]["generated_task"].is_null());
}
```

## 実装内容

### 1. service/task_service.rs

- `change_status()` -- ステータス変更、completed_at 管理
- 繰り返しタスク自動生成ロジック（Daily, Weekly, Monthly, Yearly, DaysOfWeek）
- `StatusChangeResult` 構造体

### 2. model/recurrence.rs

- 次回日付計算ロジック（chrono を使用）

### 3. cli/status.rs

- clap 引数定義（ID, STATUS）
- text 出力: 変更結果 + 繰り返し生成通知
- json 出力: task + generated_task

## 完了条件

- [ ] E2E テストが全て通る
- [ ] `done` で `completed_at` が自動設定
- [ ] `done` → 他ステータスで `completed_at` がクリア
- [ ] 繰り返しタスク（daily, weekly, monthly, yearly, days_of_week）の自動生成
- [ ] 自動生成タスクがフィールドを正しく継承
- [ ] `cancelled` では繰り返しタスクを生成しない
- [ ] `parent_id` は繰り返し生成時にコピーしない

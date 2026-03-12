# 06: status コマンド

## 概要

ステータス変更。`completed_at` の自動管理とアーカイブ機能を含む。

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

### アーカイブ動作

```rust
#[test]
fn done_archives_task() {
    // Done状態にするとarchive.jsonに移動
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "done"]);
    assert_eq!(result["data"]["task"]["status"], "done");
    assert_eq!(result["data"]["archived"], true);
    // tasks.json から消えて archive.json に移動していることを確認
    let list = todos_json(dir.path(), &["list"]);
    assert_eq!(list["data"]["count"], 0);
}

#[test]
fn done_archives_subtasks() {
    // サブタスク含めてアーカイブ
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_json(dir.path(), &["add", "子1", "--parent", &pid[..8]]);
    todos_json(dir.path(), &["add", "子2", "--parent", &pid[..8]]);
    let result = todos_json(dir.path(), &["status", &pid[..8], "done"]);
    assert_eq!(result["data"]["archived"], true);
    assert_eq!(result["data"]["archived_subtasks"], 2);
}

#[test]
fn revert_restores_from_archive() {
    // Todo/InProgressに戻すとarchiveから復元
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["status", &id[..8], "done"]).assert().success();
    let result = todos_json(dir.path(), &["status", &id[..8], "todo"]);
    assert_eq!(result["data"]["task"]["status"], "todo");
    assert!(result["data"]["task"]["completed_at"].is_null());
    // tasks.json に復元されていることを確認
    let list = todos_json(dir.path(), &["list"]);
    assert_eq!(list["data"]["count"], 1);
}

#[test]
fn cancelled_archives_task() {
    // Cancelled状態でもアーカイブ
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "テスト"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    let result = todos_json(dir.path(), &["status", &id[..8], "cancelled"]);
    assert_eq!(result["data"]["task"]["status"], "cancelled");
    assert_eq!(result["data"]["archived"], true);
}
```

## 実装内容

### 1. service/task_service.rs

- `change_status()` -- ステータス変更、completed_at 管理、アーカイブ処理
- Done/Cancelled 時: タスク（とサブタスク）を archive.json に移動
- Todo/InProgress に戻す時: archive.json から復元
- `StatusChangeResult` 構造体（`archived: bool`, `archived_subtasks: usize`）

### 2. cli/status.rs

- clap 引数定義（ID, STATUS）
- text 出力: 変更結果 + アーカイブ通知
- json 出力: task + archived + archived_subtasks

## 完了条件

- [x] E2E テストが全て通る
- [x] `done` で `completed_at` が自動設定
- [x] `done` → 他ステータスで `completed_at` がクリア
- [x] Done/Cancelled 時にタスク（とサブタスク）が archive.json に自動移動
- [x] Todo/InProgress に戻す時に archive.json から復元
- [x] `StatusChangeResult` に `archived` と `archived_subtasks` フィールド

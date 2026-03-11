# 12: TUI 一覧・詳細画面

## 概要

タスク一覧のツリー表示、詳細パネル、プロジェクトタブ、フィルタパネル。

## 依存タスク

11-tui-foundation

## テスト方針

TestBackend でレンダリング結果を検証。

```rust
#[test]
fn list_shows_tasks_with_tree() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親タスク", "-p", "high"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子タスク", "--parent", &pid[..8]]).assert().success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("親タスク"));
    assert!(content.contains("子タスク"));
}

#[test]
fn cursor_movement() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "タスク1"]).assert().success();
    todos_cmd(dir.path()).args(["add", "タスク2"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.selected_index(), 0);
    app.handle_key(key('j'));
    assert_eq!(app.selected_index(), 1);
    app.handle_key(key('k'));
    assert_eq!(app.selected_index(), 0);
}

#[test]
fn detail_panel_shows_selected_task() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト", "-d", "詳細説明", "-p", "high"]).assert().success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 120, 30);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("詳細説明"));
    assert!(content.contains("high"));
}

#[test]
fn project_tab_switching() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "A", "-P", "svc-a"]).assert().success();
    todos_cmd(dir.path()).args(["add", "B", "-P", "svc-b"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    // 初期は All
    assert_eq!(app.current_project_filter(), None);
    app.handle_key(key('l')); // 次のプロジェクト
    assert!(app.current_project_filter().is_some());
}

#[test]
fn completed_toggle() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "タスク"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["status", &id[..8], "done"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    // デフォルト: done 非表示
    assert_eq!(app.visible_task_count(), 0);
    app.handle_key(key('c')); // 完了済み表示切替
    assert_eq!(app.visible_task_count(), 1);
}

#[test]
fn parent_task_shows_subtask_count_in_detail() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path()).args(["add", "子1", "--parent", &pid[..8]]).assert().success();
    todos_cmd(dir.path()).args(["add", "子2", "--parent", &pid[..8]]).assert().success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 120, 30);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("Subtasks: 2"));
}
```

## 実装内容

### 1. tui/pages/task_list.rs

- タスク一覧のツリー表示（親→子のインデント）
- ステータスアイコン、優先度バッジ、[AI] マーカー
- カーソル移動（j/k, Up/Down）
- プロジェクトタブ（h/l, Left/Right）
- 完了済み表示切替（c）

### 2. tui/pages/task_detail.rs

- 選択中タスクの詳細パネル
- 全フィールド表示
- 親タスクの場合: Subtasks カウント表示
- サブタスクの場合: Parent 情報表示

### 3. tui/pages/filter_panel.rs

- `/` キーで開くフィルタ/検索パネル
- テキスト検索入力
- ステータス/優先度/ラベルのフィルタ選択

### 4. tui/widgets/

- `status_badge.rs` -- ステータスアイコン描画
- `priority_badge.rs` -- 優先度バッジ描画
- `help_bar.rs` -- 下部のキーバインドヘルプ

## 完了条件

- [ ] タスク一覧がツリー表示される
- [ ] サブタスクが親の下にインデント表示
- [ ] カーソル移動が動作する
- [ ] 詳細パネルに全情報が表示される
- [ ] プロジェクトタブの切り替え
- [ ] 完了済み表示の切り替え
- [ ] フィルタ/検索パネルが動作する
- [ ] ヘルプバーが表示される

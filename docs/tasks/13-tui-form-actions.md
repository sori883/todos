# 13: TUI フォーム・操作

## 概要

タスク作成/編集フォーム、ステータストグル、削除確認、データ同期。

## 依存タスク

12-tui-list-detail

## テスト方針

TestBackend + キーシーケンスで操作をシミュレート。

```rust
#[test]
fn new_task_form_opens_on_n() {
    let mut app = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), AppState::TaskForm);
}

#[test]
fn subtask_form_opens_on_s() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "親"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('s'));
    assert_eq!(app.state(), AppState::TaskForm);
    assert!(app.form_parent_id().is_some());
}

#[test]
fn edit_form_opens_on_e() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('e'));
    assert_eq!(app.state(), AppState::TaskForm);
}

#[test]
fn space_toggles_status() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    // todo -> in_progress
    app.handle_key(key(' '));
    assert_eq!(app.selected_task().unwrap().status, Status::InProgress);
    // in_progress -> done
    app.handle_key(key(' '));
    assert_eq!(app.selected_task().unwrap().status, Status::Done);
    // done -> todo
    app.handle_key(key(' '));
    assert_eq!(app.selected_task().unwrap().status, Status::Todo);
}

#[test]
fn x_cancels_task() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('x'));
    assert_eq!(app.selected_task().unwrap().status, Status::Cancelled);
}

#[test]
fn d_opens_delete_confirm() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('d'));
    assert_eq!(app.state(), AppState::DeleteConfirm);
}

#[test]
fn esc_cancels_form() {
    let mut app = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), AppState::TaskForm);
    app.handle_key(KeyCode::Esc.into());
    assert_eq!(app.state(), AppState::TaskList);
}

#[test]
fn recurrence_notification_on_done() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "繰り返し", "--recurrence", "daily"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    // todo -> in_progress -> done
    app.handle_key(key(' '));
    app.handle_key(key(' '));
    assert!(app.status_message().contains("繰り返し"));
}

#[test]
fn file_change_triggers_reload() {
    let dir = setup();
    todos_cmd(dir.path()).args(["add", "初期"]).assert().success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);
    // 外部からタスクを追加（CLI で直接）
    todos_cmd(dir.path()).args(["add", "外部追加"]).assert().success();
    // tick イベントで再読み込み
    app.handle_tick();
    assert_eq!(app.visible_task_count(), 2);
}
```

## 実装内容

### 1. tui/pages/task_form.rs

- タスク作成フォーム
  - テキスト入力: title, description
  - セレクタ: priority, label, project, parent, recurrence
  - parent セレクタ: ルートタスクのみ表示
  - parent 選択時は recurrence を never に固定
- タスク編集フォーム（既存値をプリフィル）
- サブタスク作成（parent がプリセット）
- Enter で保存、Esc でキャンセル

### 2. tui/pages/delete_confirm.rs

- 削除確認ダイアログ
- 親タスクの場合は子の数を表示
- y で削除実行、n/Esc でキャンセル

### 3. tui/app.rs 更新

- Space トグル処理（TaskService.change_status() 経由）
- x キーで cancelled に変更
- ステータスバーメッセージ（繰り返し生成通知等）
- mtime 変更検知 → 再読み込み

### 4. tui/event.rs 更新

- tick イベントで mtime チェック
- ファイル変更時にデータ再読み込み

## 完了条件

- [ ] `n` でタスク作成フォームが開く
- [ ] `s` でサブタスク作成フォーム（親がプリセット）
- [ ] `e` で編集フォーム（既存値がプリフィル）
- [ ] `Space` でステータストグル（todo → in_progress → done → todo）
- [ ] `x` で cancelled に変更
- [ ] `d` で削除確認ダイアログ
- [ ] 繰り返しタスク done 時にステータスバー通知
- [ ] 外部ファイル変更時に自動再読み込み
- [ ] Enter で保存、Esc でキャンセル

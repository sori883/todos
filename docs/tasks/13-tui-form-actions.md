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

- タスク作成フォーム（ターミナル面積の 90% を使用）
  - フィールド構成（6 フィールド）: 0=title, 1=priority, 2=label, 3=project, 4=parent, 5=content
  - テキスト入力: title, content
  - セレクタ: priority, label, project, parent
  - parent セレクタ: ルートタスクのみ表示
  - content フィールドは Unicode 対応の行折り返し
  - ブロックカーソルレンダリング
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
- ステータスバーメッセージ（アーカイブ通知等）
- mtime 変更検知 → 再読み込み

### 4. tui/event.rs 更新

- tick イベントで mtime チェック
- ファイル変更時にデータ再読み込み

## 完了条件

- [x] `n` でタスク作成フォームが開く
- [x] `s` でサブタスク作成フォーム（親がプリセット）
- [x] `e` で編集フォーム（既存値がプリフィル）
- [x] `Space` でステータストグル（todo → in_progress → done → todo）
- [x] `x` で cancelled に変更
- [x] `d` で削除確認ダイアログ
- [x] Done/Cancelled 時にアーカイブ通知
- [x] 外部ファイル変更時に自動再読み込み
- [x] Enter で保存、Esc でキャンセル

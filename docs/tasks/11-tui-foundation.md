# 11: TUI 基盤

## 概要

ターミナル初期化、App 状態管理、イベントループ、mtime ポーリングの基盤構築。

## 依存タスク

03-add-show

## テスト方針

TUI は `ratatui::backend::TestBackend` を使った統合テストで検証する。ビジネスロジックは CLI E2E テストでカバー済み。

```rust
use ratatui::backend::TestBackend;
use ratatui::Terminal;

#[test]
fn app_initializes_without_panic() {
    let dir = setup();
    let service = create_test_service(dir.path());
    let app = App::new(service);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}

#[test]
fn app_quit_on_q() {
    let app = create_test_app();
    let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let result = app.handle_key(event);
    assert!(result.should_quit);
}

#[test]
fn app_displays_task_count() {
    let dir = setup();
    // テストデータ投入
    todos_cmd(dir.path()).args(["add", "テスト"]).assert().success();
    let service = create_test_service(dir.path());
    let app = App::new(service);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(content.contains("テスト"));
}
```

## 実装内容

### 1. tui/mod.rs

- ターミナル初期化（Alternative Screen, Raw Mode）
- panic フックでターミナルリストア
- TUI メインループ起動

### 2. tui/app.rs

- `App` 構造体（`Rc<TaskService>`, UI 状態）
- `AppState` enum（TaskList, TaskForm, DeleteConfirm, FilterPanel）
- `render()` メソッド -- 現在の状態に応じた画面描画
- `handle_key()` -- キーイベントのディスパッチ

### 3. tui/event.rs

- イベントループ（crossterm のイベント待ち + tick タイマー）
- mtime ポーリング（250ms tick）
- ファイル変更検知時の再読み込みトリガー

### 4. main.rs 更新

- 引数なしで TUI を起動するように分岐

## 完了条件

- [x] `todos`（引数なし）で TUI が起動する
- [x] `q` キーで正常終了、ターミナルが復帰する
- [x] panic 時にターミナルが復帰する
- [x] イベントループが動作する（キー入力を受け付ける）
- [x] mtime ポーリングの基盤が動作する

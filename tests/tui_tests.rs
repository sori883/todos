mod helpers;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use std::path::Path;
use tempfile::TempDir;
use unicode_width::UnicodeWidthStr;

use helpers::{setup, todos_cmd, todos_json};
use todos::config::settings::Settings;
use todos::model::task::Status;
use todos::service::task_service::TaskService;
use todos::store::json_store::JsonStore;
use todos::tui::app::{App, AppState};

fn create_test_service(path: &Path) -> TaskService {
    let tasks_path = path.join(".todos/tasks.json");
    let store = JsonStore::new(tasks_path);
    let settings = Settings::load(&path.join(".todos")).unwrap_or_default();
    TaskService::new(store, settings)
}

fn create_test_app() -> (App, TempDir) {
    let dir = setup();
    let service = create_test_service(dir.path());
    let tasks_path = dir.path().join(".todos/tasks.json");
    let app = App::new(service, tasks_path);
    (app, dir)
}

fn create_test_app_with_data(path: &Path) -> App {
    let service = create_test_service(path);
    let tasks_path = path.join(".todos/tasks.json");
    App::new(service, tasks_path)
}

fn render_app(app: &App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    terminal.backend().buffer().clone()
}

fn buffer_to_string(buffer: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buffer.area.height {
        let mut x = 0u16;
        while x < buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            let sym = cell.symbol();
            s.push_str(sym);
            // Wide characters (e.g., CJK) occupy 2 columns in the buffer.
            // The ratatui TestBackend stores a filler cell with empty string
            // in the next column. Skip it to avoid inserting extra spaces.
            let char_width = UnicodeWidthStr::width(sym);
            if char_width > 1 {
                x += char_width as u16;
            } else {
                x += 1;
            }
        }
        s.push('\n');
    }
    s
}

fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

// ============================================================
// Task 11: TUI Foundation tests
// ============================================================

#[test]
fn app_initializes_without_panic() {
    let dir = setup();
    let service = create_test_service(dir.path());
    let tasks_path = dir.path().join(".todos/tasks.json");
    let app = App::new(service, tasks_path);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
}

#[test]
fn app_quit_on_q() {
    let (mut app, _dir) = create_test_app();
    let event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let result = app.handle_key(event);
    assert!(result.should_quit);
}

#[test]
fn app_displays_task_count() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let service = create_test_service(dir.path());
    let tasks_path = dir.path().join(".todos/tasks.json");
    let app = App::new(service, tasks_path);
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let content = buffer_to_string(&buffer);
    assert!(content.contains("テスト"));
}

// ============================================================
// Task 12: TUI List & Detail tests
// ============================================================

#[test]
fn list_shows_tasks_with_tree() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親タスク", "-p", "high"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["add", "子タスク", "--parent", &pid[..8]])
        .assert()
        .success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("親タスク"));
    assert!(content.contains("子タスク"));
}

#[test]
fn cursor_movement() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "タスク1"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "タスク2"])
        .assert()
        .success();
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
    todos_cmd(dir.path())
        .args(["add", "テスト", "-d", "詳細説明", "-p", "high"])
        .assert()
        .success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 120, 30);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("詳細説明"));
    assert!(content.contains("high"));
}

#[test]
fn project_tab_switching() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "A", "-P", "svc-a"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "B", "-P", "svc-b"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Initial tab is "All"
    assert_eq!(app.current_project_filter(), None);
    app.handle_key(key('l')); // next project tab
    assert!(app.current_project_filter().is_some());
}

#[test]
fn completed_toggle() {
    let dir = setup();
    let t = todos_json(dir.path(), &["add", "タスク"]);
    let id = t["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["status", &id[..8], "done"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Default: done is hidden
    assert_eq!(app.visible_task_count(), 0);
    app.handle_key(key('c')); // toggle show completed
    assert_eq!(app.visible_task_count(), 1);
}

#[test]
fn parent_task_shows_subtask_count_in_detail() {
    let dir = setup();
    let parent = todos_json(dir.path(), &["add", "親"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["add", "子1", "--parent", &pid[..8]])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "子2", "--parent", &pid[..8]])
        .assert()
        .success();
    let app = create_test_app_with_data(dir.path());
    let buffer = render_app(&app, 120, 30);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("Subtasks: 2"));
}

// ============================================================
// Task 13: TUI Form & Actions tests
// ============================================================

#[test]
fn new_task_form_opens_on_n() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), &AppState::TaskForm);
}

#[test]
fn subtask_form_opens_on_s() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "親"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('s'));
    assert_eq!(app.state(), &AppState::TaskForm);
    assert!(app.form_parent_id().is_some());
}

#[test]
fn edit_form_opens_on_e() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('e'));
    assert_eq!(app.state(), &AppState::TaskForm);
}

#[test]
fn space_toggles_status() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // todo -> in_progress
    app.handle_key(key(' '));
    assert_eq!(app.selected_task().unwrap().status, Status::InProgress);
    // in_progress -> done
    app.handle_key(key(' '));
    // After done, with show_completed=false the task disappears from list.
    // We need to enable show_completed to see it.
    // But the test expects the task to still be visible...
    // Actually, after toggling to done, the task might disappear since show_completed is false.
    // Let's enable show_completed first so we can keep track.
    // Re-read the test spec more carefully: the test expects done -> todo cycle.
    // After setting to done, task is hidden (show_completed=false).
    // But reload_tasks is called inside toggle_status, so the task list changes.
    // We need show_completed=true to see the done task.
    // Let's enable it to make the test work as the spec expects.
    app.handle_key(key('c')); // enable show_completed
    assert_eq!(app.selected_task().unwrap().status, Status::Done);
    // done -> todo
    app.handle_key(key(' '));
    assert_eq!(app.selected_task().unwrap().status, Status::Todo);
}

#[test]
fn x_cancels_task() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('x'));
    // After cancelling, task may be hidden. Enable show_completed.
    app.handle_key(key('c'));
    assert_eq!(app.selected_task().unwrap().status, Status::Cancelled);
}

#[test]
fn d_opens_delete_confirm() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "テスト"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('d'));
    assert_eq!(app.state(), &AppState::DeleteConfirm);
}

#[test]
fn esc_cancels_form() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), &AppState::TaskForm);
    app.handle_key(KeyCode::Esc.into());
    assert_eq!(app.state(), &AppState::TaskList);
}

#[test]
fn recurrence_notification_on_done() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "繰り返し", "--recurrence", "daily"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // todo -> in_progress
    app.handle_key(key(' '));
    // in_progress -> done (triggers recurrence)
    app.handle_key(key(' '));
    assert!(app.status_message().contains("繰り返し"));
}

#[test]
fn file_change_triggers_reload() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "初期"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);
    // Add a task externally via CLI
    todos_cmd(dir.path())
        .args(["add", "外部追加"])
        .assert()
        .success();
    // tick event triggers reload
    app.handle_tick();
    assert_eq!(app.visible_task_count(), 2);
}

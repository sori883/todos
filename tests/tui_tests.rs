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
use todos::store::sqlite_store::SqliteStore;
use todos::tui::app::{App, AppState, FormMode};

fn create_test_service(path: &Path) -> TaskService {
    let db_path = path.join(".todos/todos.db");
    let conn = SqliteStore::open(&db_path).unwrap();
    let store = SqliteStore::new(conn.clone(), "tasks").unwrap();
    let archive_store = SqliteStore::new(conn, "archive").unwrap();
    let settings = Settings::load(&path.join(".todos")).unwrap_or_default();
    TaskService::new(store, settings, archive_store)
}

fn create_test_app() -> (App, TempDir) {
    let dir = setup();
    let service = create_test_service(dir.path());
    let db_path = dir.path().join(".todos/todos.db");
    let app = App::new(service, db_path);
    (app, dir)
}

fn create_test_app_with_data(path: &Path) -> App {
    let service = create_test_service(path);
    let db_path = path.join(".todos/todos.db");
    App::new(service, db_path)
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
    let db_path = dir.path().join(".todos/todos.db");
    let app = App::new(service, db_path);
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
    let db_path = dir.path().join(".todos/todos.db");
    let app = App::new(service, db_path);
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
    // Done task is archived, so it's not in the active list
    assert_eq!(app.visible_task_count(), 0);
    // With show_completed, archived tasks are included
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
    assert!(content.contains("Subtasks") && content.contains("2"));
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
    todos_cmd(dir.path()).args(["add", "親"]).assert().success();
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
    // in_progress -> done: task gets archived and disappears from the list
    app.handle_key(key(' '));
    // Task is now archived, so the list is empty
    assert_eq!(app.visible_task_count(), 0);
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
    // After cancelling, task is archived and disappears from the list
    assert_eq!(app.visible_task_count(), 0);
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

// ============================================================
// Reproduction tests for TUI improvement issues
// ============================================================

fn type_string(app: &mut App, s: &str) {
    for c in s.chars() {
        app.handle_key(key(c));
    }
}

#[test]
fn repro_create_task_via_form() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), &AppState::TaskForm);
    type_string(&mut app, "FormTask");
    app.handle_key(KeyCode::Enter.into());
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 1);
}

#[test]
fn repro_create_subtask_via_form() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Parent"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);
    app.handle_key(key('s'));
    assert_eq!(app.state(), &AppState::TaskForm);
    assert!(app.form_parent_id().is_some());
    type_string(&mut app, "Child");
    app.handle_key(KeyCode::Enter.into());
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 2);
}

#[test]
fn repro_small_terminal_no_crash() {
    let (app, _dir) = create_test_app();
    for (w, h) in [(10, 5), (5, 3), (1, 1), (3, 1), (80, 1), (1, 24)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            terminal.draw(|f| app.render(f)).unwrap();
        }));
        assert!(result.is_ok(), "Crashed at terminal size {}x{}", w, h);
    }
}

#[test]
fn repro_small_terminal_with_tasks_no_crash() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Task1"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "Task2", "-P", "myproject"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Also open form
    app.handle_key(key('n'));
    for (w, h) in [(10, 5), (5, 3), (1, 1), (3, 1), (80, 1), (1, 24), (20, 10)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            terminal.draw(|f| app.render(f)).unwrap();
        }));
        assert!(
            result.is_ok(),
            "Crashed at terminal size {}x{} with form",
            w,
            h
        );
    }
    // Also test delete confirm dialog
    app.handle_key(KeyCode::Esc.into());
    app.handle_key(key('d'));
    for (w, h) in [(10, 5), (5, 3), (20, 10)] {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            terminal.draw(|f| app.render(f)).unwrap();
        }));
        assert!(
            result.is_ok(),
            "Crashed at terminal size {}x{} with delete confirm",
            w,
            h
        );
    }
}

// ============================================================
// TUI Improvement: project indicator tests
// ============================================================

#[test]
fn project_indicator_shows_current_project() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "A", "-P", "alpha"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "B", "-P", "beta"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Initial: "All" project
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("All"),
        "Should show 'All' as current project"
    );

    // Switch to next project
    app.handle_key(key('l'));
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("alpha"),
        "Should show 'alpha' as current project"
    );
}

// ============================================================
// TUI Improvement: parent selector in form
// ============================================================

#[test]
fn form_parent_selector_shows_available_parents() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "ParentA"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "ParentB"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Open new task form
    app.handle_key(key('n'));
    let form = app.form().unwrap();
    // Should have 2 available parents
    assert_eq!(form.available_parents.len(), 2);
    assert_eq!(form.parent_index, 0); // none selected by default
}

#[test]
fn form_create_subtask_via_parent_selector() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Root"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);

    // Open new task form
    app.handle_key(key('n'));
    // Type title
    type_string(&mut app, "Child via selector");
    // Tab to parent field (field 4): tab 4 times
    for _ in 0..4 {
        app.handle_key(KeyCode::Tab.into());
    }
    // Select parent with Right arrow
    app.handle_key(KeyCode::Right.into());
    // Verify parent is selected
    assert!(app.form_parent_id().is_some());
    // Save
    app.handle_key(KeyCode::Enter.into());
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 2);
    // The second task should be a subtask
    let tasks = app.tasks();
    let child = tasks
        .iter()
        .find(|t| t.title == "Child via selector")
        .unwrap();
    assert!(child.parent_id.is_some());
}

#[test]
fn form_parent_field_renders_in_form() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Root"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('n'));
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("Parent"),
        "Form should contain Parent field"
    );
}

#[test]
fn subtask_form_parent_locked() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Root"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Open subtask form
    app.handle_key(key('s'));
    let form = app.form().unwrap();
    assert_eq!(form.mode, FormMode::Subtask);
    // Parent should be pre-selected (index > 0)
    assert!(form.parent_index > 0);
    let buffer = render_app(&app, 80, 24);
    let content = buffer_to_string(&buffer);
    assert!(
        content.contains("Parent (locked)"),
        "Subtask form should show locked parent"
    );
}

// ============================================================
// TUI Improvement: form error preserves data
// ============================================================

#[test]
fn form_save_error_preserves_form_data() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    // Try to save with empty title
    app.handle_key(KeyCode::Enter.into());
    // Should still be in form state
    assert_eq!(app.state(), &AppState::TaskForm);
    assert!(app.status_message().contains("empty"));
}

// ============================================================
// TUI Improvement: minimum terminal size
// ============================================================

#[test]
fn tiny_terminal_shows_too_small_message() {
    let (app, _dir) = create_test_app();
    let buffer = render_app(&app, 19, 4);
    let content = buffer_to_string(&buffer);
    assert!(content.contains("Terminal too small"));
}

// ============================================================
// Additional tests: delete confirm actions
// ============================================================

#[test]
fn delete_confirm_y_deletes_task() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "ToDelete"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);
    // Open delete confirm
    app.handle_key(key('d'));
    assert_eq!(app.state(), &AppState::DeleteConfirm);
    // Confirm deletion with 'y'
    app.handle_key(key('y'));
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 0);
}

#[test]
fn delete_confirm_n_cancels() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Keep"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 1);
    app.handle_key(key('d'));
    assert_eq!(app.state(), &AppState::DeleteConfirm);
    // Cancel with 'n'
    app.handle_key(key('n'));
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 1);
}

#[test]
fn delete_confirm_esc_cancels() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "Keep"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('d'));
    assert_eq!(app.state(), &AppState::DeleteConfirm);
    app.handle_key(KeyCode::Esc.into());
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 1);
}

// ============================================================
// Additional tests: edit form pre-fill
// ============================================================

#[test]
fn edit_form_prefills_existing_values() {
    let dir = setup();
    todos_cmd(dir.path())
        .args([
            "add", "MyTitle", "-d", "MyDesc", "-p", "high", "-l", "bug", "-P", "proj1",
        ])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    app.handle_key(key('e'));
    assert_eq!(app.state(), &AppState::TaskForm);
    let form = app.form().unwrap();
    assert_eq!(form.mode, FormMode::Edit);
    assert_eq!(form.title, "MyTitle");
    assert_eq!(form.content, "MyDesc");
    assert_eq!(form.priority_index, 3); // high is index 3 in PRIORITIES
    assert!(form.label_index > 0); // bug should be selected
    assert_eq!(form.project, "proj1");
}

// ============================================================
// Additional tests: parent selector excludes self in edit
// ============================================================

#[test]
fn edit_form_parent_excludes_self() {
    let dir = setup();
    todos_cmd(dir.path())
        .args(["add", "TaskA"])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "TaskB"])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    // Edit first task
    app.handle_key(key('e'));
    let form = app.form().unwrap();
    // Available parents should contain TaskB but not TaskA (self)
    let titles: Vec<&str> = form
        .available_parents
        .iter()
        .map(|(_, t)| t.as_str())
        .collect();
    assert!(
        titles.contains(&"TaskB"),
        "TaskB should be available as parent"
    );
    assert!(
        !titles.contains(&"TaskA"),
        "TaskA (self) should NOT be available as parent"
    );
}

// ============================================================
// Additional tests: form Tab/BackTab navigation
// ============================================================

#[test]
fn form_tab_cycles_all_fields() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    // Start at field 0
    assert_eq!(app.form().unwrap().focused_field, 0);
    // Tab through all 6 fields and back to 0
    for expected in 1..=5 {
        app.handle_key(KeyCode::Tab.into());
        assert_eq!(app.form().unwrap().focused_field, expected);
    }
    app.handle_key(KeyCode::Tab.into());
    assert_eq!(
        app.form().unwrap().focused_field,
        0,
        "Should wrap around to field 0"
    );
}

#[test]
fn form_backtab_cycles_all_fields() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.form().unwrap().focused_field, 0);
    // BackTab from 0 should go to last field (5)
    app.handle_key(KeyCode::BackTab.into());
    assert_eq!(
        app.form().unwrap().focused_field,
        5,
        "BackTab from 0 should wrap to last field"
    );
    // BackTab again should go to 4
    app.handle_key(KeyCode::BackTab.into());
    assert_eq!(app.form().unwrap().focused_field, 4);
}

// ============================================================
// Additional tests: delete parent with cascading subtasks
// ============================================================

#[test]
fn delete_parent_cascades_in_tui() {
    let dir = setup();
    let parent = helpers::todos_json(dir.path(), &["add", "Parent"]);
    let pid = parent["data"]["task"]["id"].as_str().unwrap();
    todos_cmd(dir.path())
        .args(["add", "Child1", "--parent", &pid[..8]])
        .assert()
        .success();
    todos_cmd(dir.path())
        .args(["add", "Child2", "--parent", &pid[..8]])
        .assert()
        .success();
    let mut app = create_test_app_with_data(dir.path());
    assert_eq!(app.visible_task_count(), 3); // parent + 2 children
    // Select parent (should be first), delete
    app.handle_key(key('d'));
    assert_eq!(app.state(), &AppState::DeleteConfirm);
    app.handle_key(key('y'));
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 0); // all deleted
}

// ============================================================
// Multiline content tests
// ============================================================

fn alt_enter() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::ALT)
}

fn ctrl_j() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL)
}

#[test]
fn content_multiline_input_via_alt_enter() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    assert_eq!(app.state(), &AppState::TaskForm);
    // Type title
    type_string(&mut app, "MultilineTask");
    // Tab to content field (field 5)
    for _ in 0..5 {
        app.handle_key(KeyCode::Tab.into());
    }
    // Type first line
    type_string(&mut app, "line1");
    // Insert newline via Alt+Enter
    app.handle_key(alt_enter());
    // Type second line
    type_string(&mut app, "line2");
    // Insert newline via Ctrl+J
    app.handle_key(ctrl_j());
    // Type third line
    type_string(&mut app, "line3");
    // Verify the content contains newlines
    let form = app.form().unwrap();
    assert_eq!(form.content, "line1\nline2\nline3");
    // Save and verify task was created
    app.handle_key(KeyCode::Enter.into());
    assert_eq!(app.state(), &AppState::TaskList);
    assert_eq!(app.visible_task_count(), 1);
    // Verify the task's content has newlines
    let task = app.selected_task().unwrap();
    assert_eq!(task.content.as_deref(), Some("line1\nline2\nline3"));
}

#[test]
fn content_multiline_display_in_detail() {
    let dir = setup();
    // Create a task with multiline content via CLI
    todos_cmd(dir.path())
        .args(["add", "Detail", "-d", "first\nsecond\nthird"])
        .assert()
        .success();
    let app = create_test_app_with_data(dir.path());
    // Render with wide terminal to get detail panel
    let buffer = render_app(&app, 120, 30);
    let content = buffer_to_string(&buffer);
    // The detail panel should show each line separately
    // Content section shows lines directly (no "Content:" label in new design)
    assert!(
        content.contains("first"),
        "Should show first line of content"
    );
    assert!(
        content.contains("second"),
        "Should show second line of content"
    );
    assert!(
        content.contains("third"),
        "Should show third line of content"
    );
}

#[test]
fn content_cursor_navigation() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    // Tab to content field (field 5)
    for _ in 0..5 {
        app.handle_key(KeyCode::Tab.into());
    }
    // Type first line
    type_string(&mut app, "abc");
    // Insert newline
    app.handle_key(alt_enter());
    // Type second line
    type_string(&mut app, "defgh");
    // Insert newline
    app.handle_key(alt_enter());
    // Type third line
    type_string(&mut app, "ij");
    // Cursor should be at row 2, col 2
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 2);
    assert_eq!(form.content_cursor_col, 2);
    // Move up
    app.handle_key(KeyCode::Up.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 1);
    assert_eq!(form.content_cursor_col, 2); // stays at col 2 (defgh has 5 chars)
    // Move up again
    app.handle_key(KeyCode::Up.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 0);
    assert_eq!(form.content_cursor_col, 2); // stays at col 2 (abc has 3 chars)
    // Move up at top should stay at row 0
    app.handle_key(KeyCode::Up.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 0);
    // Move down
    app.handle_key(KeyCode::Down.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 1);
    // Move down again
    app.handle_key(KeyCode::Down.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 2);
    // Move down at bottom should stay
    app.handle_key(KeyCode::Down.into());
    let form = app.form().unwrap();
    assert_eq!(form.content_cursor_row, 2);
}

#[test]
fn content_backspace_joins_lines() {
    let (mut app, _dir) = create_test_app();
    app.handle_key(key('n'));
    // Tab to content field (field 5)
    for _ in 0..5 {
        app.handle_key(KeyCode::Tab.into());
    }
    // Type first line
    type_string(&mut app, "hello");
    // Insert newline
    app.handle_key(alt_enter());
    // Type second line
    type_string(&mut app, "world");
    // Content should be "hello\nworld"
    let form = app.form().unwrap();
    assert_eq!(form.content, "hello\nworld");
    assert_eq!(form.content_cursor_row, 1);
    assert_eq!(form.content_cursor_col, 5);
    // Delete all chars on second line
    for _ in 0..5 {
        app.handle_key(KeyCode::Backspace.into());
    }
    // Now cursor is at beginning of second line
    let form = app.form().unwrap();
    assert_eq!(form.content, "hello\n");
    assert_eq!(form.content_cursor_row, 1);
    assert_eq!(form.content_cursor_col, 0);
    // One more backspace should join lines
    app.handle_key(KeyCode::Backspace.into());
    let form = app.form().unwrap();
    assert_eq!(form.content, "hello");
    assert_eq!(form.content_cursor_row, 0);
    assert_eq!(form.content_cursor_col, 5); // cursor at end of "hello"
}

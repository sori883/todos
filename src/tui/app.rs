use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;

use crate::model::filter::TaskFilter;
use crate::model::task::{Priority, Status, Task, TaskId};
use crate::service::task_service::TaskService;

use super::pages;

#[path = "app_form.rs"]
mod form_handlers;

// Priority options for form selector
pub const PRIORITIES: &[Priority] = &[
    Priority::None,
    Priority::Low,
    Priority::Medium,
    Priority::High,
    Priority::Critical,
];

#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    TaskList,
    TaskForm,
    DeleteConfirm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FormMode {
    New,
    Edit,
    Subtask,
}

pub struct TaskFormData {
    pub mode: FormMode,
    pub title: String,
    pub title_cursor: usize,
    pub content: String,
    pub content_cursor_row: usize,
    pub content_cursor_col: usize,
    pub priority_index: usize,
    pub label_index: usize, // 0 = none, 1+ = labels
    pub project: String,
    pub project_cursor: usize,
    pub parent_index: usize, // 0 = none, 1+ = available_parents
    pub available_parents: Vec<(TaskId, String)>, // (id, title) of root tasks
    pub editing_task_id: Option<TaskId>,
    pub focused_field: usize,
    pub available_labels: Vec<String>, // cached labels from settings
}

impl TaskFormData {
    /// Get the selected parent task ID (None if index is 0).
    pub fn selected_parent_id(&self) -> Option<TaskId> {
        if self.parent_index == 0 {
            None
        } else {
            self.available_parents
                .get(self.parent_index - 1)
                .map(|(id, _)| *id)
        }
    }
}

pub struct KeyResult {
    pub should_quit: bool,
}

pub struct App {
    service: TaskService,
    db_path: PathBuf,
    state: AppState,
    tasks: Vec<Task>,
    selected_index: usize,
    project_tabs: Vec<Option<String>>, // None = "All", Some("project-name")
    current_tab: usize,
    show_completed: bool,
    status_message: String,
    status_message_time: Option<Instant>,
    last_mtime: Option<SystemTime>,
    form: Option<TaskFormData>,
    delete_target: Option<Task>,
}

impl App {
    pub fn new(service: TaskService, db_path: PathBuf) -> Self {
        let mut app = Self {
            service,
            db_path,
            state: AppState::TaskList,
            tasks: Vec::new(),
            selected_index: 0,
            project_tabs: vec![None],
            current_tab: 0,
            show_completed: false,
            status_message: String::new(),
            status_message_time: None,
            last_mtime: None,
            form: None,
            delete_target: None,
        };
        app.update_mtime();
        app.reload_tasks();
        app
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn visible_task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected_index)
    }

    pub fn current_project_filter(&self) -> Option<&str> {
        self.project_tabs
            .get(self.current_tab)
            .and_then(|opt| opt.as_deref())
    }

    pub fn status_message(&self) -> &str {
        &self.status_message
    }

    pub fn form_parent_id(&self) -> Option<TaskId> {
        self.form.as_ref().and_then(|f| f.selected_parent_id())
    }

    /// Render the current state to the given frame.
    pub fn render(&self, frame: &mut Frame) {
        match self.state {
            AppState::TaskList => {
                pages::task_list::render(frame, self);
            }
            AppState::TaskForm => {
                // Render the list behind the form, then overlay the form
                pages::task_list::render(frame, self);
                if let Some(ref form) = self.form {
                    pages::task_form::render(frame, form);
                }
            }
            AppState::DeleteConfirm => {
                pages::task_list::render(frame, self);
                if let Some(ref target) = self.delete_target {
                    let subtask_count = self.count_subtasks(target.id);
                    pages::delete_confirm::render(frame, target, subtask_count);
                }
            }
        }
    }

    /// Handle a key event and return whether the app should quit.
    pub fn handle_key(&mut self, key: KeyEvent) -> KeyResult {
        // Clear expired status message (after 5 seconds)
        if let Some(time) = self.status_message_time
            && time.elapsed().as_secs() >= 5
        {
            self.status_message.clear();
            self.status_message_time = None;
        }

        match self.state {
            AppState::TaskList => self.handle_key_list(key),
            AppState::TaskForm => self.handle_key_form(key),
            AppState::DeleteConfirm => self.handle_key_delete_confirm(key),
        }
    }

    /// Handle tick events for mtime polling.
    pub fn handle_tick(&mut self) {
        // Clear expired status message
        if let Some(time) = self.status_message_time
            && time.elapsed().as_secs() >= 5
        {
            self.status_message.clear();
            self.status_message_time = None;
        }

        // Check mtime (DB file + WAL file, since WAL mode writes to the -wal file)
        let current_mtime = self.latest_db_mtime();
        if let Some(mtime) = current_mtime
            && self.last_mtime.is_some_and(|last| mtime != last)
        {
            self.reload_tasks();
            self.last_mtime = Some(mtime);
        }
    }

    /// Reload tasks from the service, applying current filters.
    pub fn reload_tasks(&mut self) {
        let filter = TaskFilter {
            project: self.current_project_filter().map(|s| s.to_string()),
            include_done: self.show_completed,
            include_cancelled: self.show_completed,
            ..Default::default()
        };

        match self.service.list_tasks_tree(&filter) {
            Ok(mut tasks) => {
                // When show_completed is on, also include archived (done/cancelled) tasks
                if self.show_completed {
                    let archive_filter = TaskFilter {
                        project: self.current_project_filter().map(|s| s.to_string()),
                        include_done: true,
                        include_cancelled: true,
                        ..Default::default()
                    };
                    if let Ok(archived) = self.service.list_archive(&archive_filter) {
                        tasks.extend(archived);
                    }
                }
                self.tasks = tasks;
                // Adjust selected_index if it's out of bounds
                if !self.tasks.is_empty() && self.selected_index >= self.tasks.len() {
                    self.selected_index = self.tasks.len() - 1;
                }
                if self.tasks.is_empty() {
                    self.selected_index = 0;
                }
            }
            Err(_) => {
                self.tasks = Vec::new();
                self.selected_index = 0;
            }
        }

        // Update project tabs
        self.update_project_tabs();
    }

    // --- Internal helpers ---

    fn update_mtime(&mut self) {
        self.last_mtime = self.latest_db_mtime();
    }

    /// Return the latest mtime across the DB file and its WAL file.
    fn latest_db_mtime(&self) -> Option<SystemTime> {
        let db_mtime = std::fs::metadata(&self.db_path)
            .ok()
            .and_then(|m| m.modified().ok());
        let wal_path = self.db_path.with_extension("db-wal");
        let wal_mtime = std::fs::metadata(&wal_path)
            .ok()
            .and_then(|m| m.modified().ok());
        match (db_mtime, wal_mtime) {
            (Some(d), Some(w)) => Some(d.max(w)),
            (d, w) => d.or(w),
        }
    }

    fn update_project_tabs(&mut self) {
        let all_filter = TaskFilter {
            include_done: true,
            include_cancelled: true,
            ..Default::default()
        };

        let mut projects: Vec<String> = Vec::new();
        if let Ok(all_tasks) = self.service.list_tasks(&all_filter) {
            for task in &all_tasks {
                if let Some(ref proj) = task.project
                    && !projects.contains(proj)
                {
                    projects.push(proj.clone());
                }
            }
        }
        projects.sort();

        let mut tabs: Vec<Option<String>> = vec![None]; // "All" tab
        for proj in projects {
            tabs.push(Some(proj));
        }
        self.project_tabs = tabs;

        // Ensure current_tab is valid
        if self.current_tab >= self.project_tabs.len() {
            self.current_tab = 0;
        }
    }

    fn set_status_message(&mut self, msg: String) {
        self.status_message = msg;
        self.status_message_time = Some(Instant::now());
    }

    fn count_subtasks(&self, parent_id: TaskId) -> usize {
        match self.service.get_subtasks(parent_id) {
            Ok(children) => children.len(),
            Err(_) => 0,
        }
    }

    fn task_id_prefix(&self, task: &Task) -> String {
        let id_str = task.id.to_string();
        id_str[..8.min(id_str.len())].to_string()
    }

    fn next_status(current: Status) -> &'static str {
        match current {
            Status::Todo => "in_progress",
            Status::InProgress => "done",
            Status::Done => "todo",
            Status::Cancelled => "todo",
        }
    }

    // --- Key handlers ---

    fn handle_key_list(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            KeyCode::Char('q') => {
                return KeyResult { should_quit: true };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.tasks.is_empty() && self.selected_index < self.tasks.len() - 1 {
                    self.selected_index += 1;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.current_tab > 0 {
                    self.current_tab -= 1;
                } else {
                    self.current_tab = self.project_tabs.len() - 1;
                }
                self.selected_index = 0;
                self.reload_tasks();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.current_tab < self.project_tabs.len() - 1 {
                    self.current_tab += 1;
                } else {
                    self.current_tab = 0;
                }
                self.selected_index = 0;
                self.reload_tasks();
            }
            KeyCode::Char('c') => {
                self.show_completed = !self.show_completed;
                self.reload_tasks();
            }
            KeyCode::Char('n') => {
                self.open_new_task_form();
            }
            KeyCode::Char('e') => {
                self.open_edit_form();
            }
            KeyCode::Char('s') => {
                self.open_subtask_form();
            }
            KeyCode::Char(' ') => {
                self.toggle_status();
            }
            KeyCode::Char('x') => {
                self.cancel_task();
            }
            KeyCode::Char('d') => {
                self.open_delete_confirm();
            }
            KeyCode::Esc => {
                self.status_message.clear();
                self.status_message_time = None;
            }
            _ => {}
        }
        KeyResult { should_quit: false }
    }

    fn handle_key_delete_confirm(&mut self, key: KeyEvent) -> KeyResult {
        match key.code {
            KeyCode::Char('y') => {
                if let Some(ref target) = self.delete_target.clone() {
                    let prefix = self.task_id_prefix(target);
                    match self.service.delete_task(&prefix) {
                        Ok(result) => {
                            let msg = if result.deleted_subtasks > 0 {
                                format!(
                                    "Deleted '{}' and {} subtask(s)",
                                    result.task.title, result.deleted_subtasks
                                )
                            } else {
                                format!("Deleted '{}'", result.task.title)
                            };
                            self.set_status_message(msg);
                        }
                        Err(e) => {
                            self.set_status_message(format!("Delete failed: {e}"));
                        }
                    }
                }
                self.delete_target = None;
                self.state = AppState::TaskList;
                self.update_mtime();

                self.reload_tasks();
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.delete_target = None;
                self.state = AppState::TaskList;
            }
            _ => {}
        }
        KeyResult { should_quit: false }
    }

    // --- Task actions ---

    fn toggle_status(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let prefix = self.task_id_prefix(&task);
            let new_status = Self::next_status(task.status);
            match self.service.change_status(&prefix, new_status) {
                Ok(result) => {
                    if result.archived {
                        let msg = if result.archived_subtasks > 0 {
                            format!(
                                "サブタスク{}件と共にアーカイブしました",
                                result.archived_subtasks
                            )
                        } else {
                            "アーカイブしました".to_string()
                        };
                        self.set_status_message(msg);
                    }
                }
                Err(e) => {
                    self.set_status_message(format!("Status change failed: {e}"));
                }
            }
            self.update_mtime();

            self.reload_tasks();
        }
    }

    fn cancel_task(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let prefix = self.task_id_prefix(&task);
            match self.service.change_status(&prefix, "cancelled") {
                Ok(result) => {
                    let mut msg = format!("Cancelled '{}'", task.title);
                    if result.archived {
                        if result.archived_subtasks > 0 {
                            msg.push_str(&format!(
                                " | サブタスク{}件と共にアーカイブしました",
                                result.archived_subtasks
                            ));
                        } else {
                            msg.push_str(" | アーカイブしました");
                        }
                    }
                    self.set_status_message(msg);
                }
                Err(e) => {
                    self.set_status_message(format!("Cancel failed: {e}"));
                }
            }
            self.update_mtime();

            self.reload_tasks();
        }
    }

    fn open_delete_confirm(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            self.delete_target = Some(task);
            self.state = AppState::DeleteConfirm;
        }
    }

    // --- Public accessors for rendering ---

    pub fn tasks(&self) -> &[Task] {
        &self.tasks
    }

    pub fn project_tabs(&self) -> &[Option<String>] {
        &self.project_tabs
    }

    pub fn current_tab(&self) -> usize {
        self.current_tab
    }

    pub fn show_completed(&self) -> bool {
        self.show_completed
    }

    pub fn form(&self) -> Option<&TaskFormData> {
        self.form.as_ref()
    }

    pub fn delete_target(&self) -> Option<&Task> {
        self.delete_target.as_ref()
    }

    /// Get the service reference for detail panel queries.
    pub fn service(&self) -> &TaskService {
        &self.service
    }
}

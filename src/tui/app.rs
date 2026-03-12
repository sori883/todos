use std::path::PathBuf;
use std::time::{Instant, SystemTime};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;

use crate::model::filter::TaskFilter;
use crate::model::recurrence::Recurrence;
use crate::model::task::{CreatedBy, Priority, Status, Task, TaskId};
use crate::service::task_service::TaskService;

use super::pages;

// Priority options for form selector
pub const PRIORITIES: &[Priority] = &[
    Priority::None,
    Priority::Low,
    Priority::Medium,
    Priority::High,
    Priority::Critical,
];

// Recurrence options for form selector
pub const RECURRENCES: &[&str] = &[
    "never", "daily", "weekly", "monthly", "yearly",
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
    pub description: String,
    pub priority_index: usize,
    pub label_index: usize, // 0 = none, 1+ = labels
    pub project: String,
    pub recurrence_index: usize,
    pub parent_id: Option<TaskId>,
    pub editing_task_id: Option<TaskId>,
    pub focused_field: usize,
    pub available_labels: Vec<String>, // cached labels from settings
}

pub struct KeyResult {
    pub should_quit: bool,
}

pub struct App {
    service: TaskService,
    tasks_path: PathBuf,
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
    pub fn new(service: TaskService, tasks_path: PathBuf) -> Self {
        let mut app = Self {
            service,
            tasks_path,
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
        self.form.as_ref().and_then(|f| f.parent_id)
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
        if let Some(time) = self.status_message_time {
            if time.elapsed().as_secs() >= 5 {
                self.status_message.clear();
                self.status_message_time = None;
            }
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
        if let Some(time) = self.status_message_time {
            if time.elapsed().as_secs() >= 5 {
                self.status_message.clear();
                self.status_message_time = None;
            }
        }

        // Check mtime
        if let Ok(metadata) = std::fs::metadata(&self.tasks_path) {
            if let Ok(mtime) = metadata.modified() {
                if self.last_mtime.is_some_and(|last| mtime != last) {
                    self.service_invalidate_cache();
                    self.reload_tasks();
                    self.last_mtime = Some(mtime);
                }
            }
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
            Ok(tasks) => {
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
        if let Ok(metadata) = std::fs::metadata(&self.tasks_path) {
            if let Ok(mtime) = metadata.modified() {
                self.last_mtime = Some(mtime);
            }
        }
    }

    fn service_invalidate_cache(&self) {
        // Access the store's invalidate_cache through the service.
        // We need to expose this - for now we re-create by forcing a reload.
        // The JsonStore has invalidate_cache, but TaskService doesn't expose it.
        // We'll add a method to TaskService.
        self.service.invalidate_cache();
    }

    fn update_project_tabs(&mut self) {
        // Scan all tasks (including done/cancelled) for unique projects
        let all_filter = TaskFilter {
            include_done: true,
            include_cancelled: true,
            ..Default::default()
        };

        let mut projects: Vec<String> = Vec::new();
        if let Ok(all_tasks) = self.service.list_tasks(&all_filter) {
            for task in &all_tasks {
                if let Some(ref proj) = task.project {
                    if !projects.contains(proj) {
                        projects.push(proj.clone());
                    }
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
        // Use at least 8 chars of the task ID for prefix lookup
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

    // --- Key handlers per state ---

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
                // Previous project tab
                if self.current_tab > 0 {
                    self.current_tab -= 1;
                } else {
                    self.current_tab = self.project_tabs.len() - 1;
                }
                self.selected_index = 0;
                self.reload_tasks();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                // Next project tab
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

    fn handle_key_form(&mut self, key: KeyEvent) -> KeyResult {
        if let Some(ref mut form) = self.form {
            match key.code {
                KeyCode::Esc => {
                    self.form = None;
                    self.state = AppState::TaskList;
                }
                KeyCode::Enter => {
                    self.save_form();
                }
                KeyCode::Tab => {
                    // Move to next field
                    form.focused_field = (form.focused_field + 1) % 6;
                }
                KeyCode::BackTab => {
                    // Move to previous field
                    if form.focused_field == 0 {
                        form.focused_field = 5;
                    } else {
                        form.focused_field -= 1;
                    }
                }
                KeyCode::Backspace => {
                    match form.focused_field {
                        0 => { form.title.pop(); }
                        1 => { form.description.pop(); }
                        4 => { form.project.pop(); }
                        _ => {}
                    }
                }
                KeyCode::Left => {
                    match form.focused_field {
                        2 => {
                            // Priority selector - decrease
                            if form.priority_index > 0 {
                                form.priority_index -= 1;
                            }
                        }
                        3 => {
                            // Label selector - decrease
                            if form.label_index > 0 {
                                form.label_index -= 1;
                            }
                        }
                        5 => {
                            // Recurrence selector - decrease (skip if subtask)
                            if form.mode != FormMode::Subtask && form.recurrence_index > 0 {
                                form.recurrence_index -= 1;
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Right => {
                    match form.focused_field {
                        2 => {
                            // Priority selector - increase
                            if form.priority_index < PRIORITIES.len() - 1 {
                                form.priority_index += 1;
                            }
                        }
                        3 => {
                            // Label selector - increase
                            let max = form.available_labels.len(); // 0=none + labels
                            if form.label_index < max {
                                form.label_index += 1;
                            }
                        }
                        5 => {
                            // Recurrence selector - increase (skip if subtask)
                            if form.mode != FormMode::Subtask
                                && form.recurrence_index < RECURRENCES.len() - 1
                            {
                                form.recurrence_index += 1;
                            }
                        }
                        _ => {}
                    }
                }
                KeyCode::Char(c) => {
                    // Only allow typing in text fields
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        // Ignore ctrl combos
                    } else {
                        match form.focused_field {
                            0 => form.title.push(c),
                            1 => form.description.push(c),
                            4 => form.project.push(c),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
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
                self.service_invalidate_cache();
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

    // --- Form operations ---

    fn open_new_task_form(&mut self) {
        let labels = self.get_available_labels();
        self.form = Some(TaskFormData {
            mode: FormMode::New,
            title: String::new(),
            description: String::new(),
            priority_index: 0,
            label_index: 0,
            project: String::new(),
            recurrence_index: 0,
            parent_id: None,
            editing_task_id: None,
            focused_field: 0,
            available_labels: labels,
        });
        self.state = AppState::TaskForm;
    }

    fn open_edit_form(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let labels = self.get_available_labels();
            let priority_index = PRIORITIES
                .iter()
                .position(|p| *p == task.priority)
                .unwrap_or(0);
            let label_index = match &task.label {
                None => 0,
                Some(l) => labels.iter().position(|lab| lab == l).map_or(0, |i| i + 1),
            };
            let recurrence_index = match &task.recurrence {
                Recurrence::Never => 0,
                Recurrence::Daily => 1,
                Recurrence::Weekly => 2,
                Recurrence::Monthly => 3,
                Recurrence::Yearly => 4,
                _ => 0,
            };

            self.form = Some(TaskFormData {
                mode: FormMode::Edit,
                title: task.title.clone(),
                description: task.description.clone().unwrap_or_default(),
                priority_index,
                label_index,
                project: task.project.clone().unwrap_or_default(),
                recurrence_index,
                parent_id: task.parent_id,
                editing_task_id: Some(task.id),
                focused_field: 0,
                available_labels: labels,
            });
            self.state = AppState::TaskForm;
        }
    }

    fn open_subtask_form(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            // If selected task is already a subtask, use its parent
            let parent_id = if task.parent_id.is_some() {
                task.parent_id
            } else {
                Some(task.id)
            };

            let labels = self.get_available_labels();
            self.form = Some(TaskFormData {
                mode: FormMode::Subtask,
                title: String::new(),
                description: String::new(),
                priority_index: 0,
                label_index: 0,
                project: task.project.clone().unwrap_or_default(),
                recurrence_index: 0, // locked to never
                parent_id,
                editing_task_id: None,
                focused_field: 0,
                available_labels: labels,
            });
            self.state = AppState::TaskForm;
        }
    }

    fn save_form(&mut self) {
        let form = match self.form.take() {
            Some(f) => f,
            None => return,
        };

        if form.title.trim().is_empty() {
            self.set_status_message("Title cannot be empty".to_string());
            self.form = Some(form);
            return;
        }

        let priority = PRIORITIES[form.priority_index];
        let label = if form.label_index == 0 {
            None
        } else {
            form.available_labels.get(form.label_index - 1).cloned()
        };
        let project = if form.project.trim().is_empty() {
            None
        } else {
            Some(form.project.trim().to_string())
        };
        let recurrence_str = RECURRENCES[form.recurrence_index];
        let recurrence = Recurrence::parse(recurrence_str).unwrap_or(Recurrence::Never);
        let description = if form.description.trim().is_empty() {
            None
        } else {
            Some(form.description.trim().to_string())
        };

        match form.mode {
            FormMode::New => {
                match self.service.add_task(
                    form.title.trim().to_string(),
                    description,
                    priority,
                    CreatedBy::Human,
                    label,
                    project,
                    None,
                    recurrence,
                ) {
                    Ok(_) => {
                        self.set_status_message("Task created".to_string());
                    }
                    Err(e) => {
                        self.set_status_message(format!("Failed to create task: {e}"));
                    }
                }
            }
            FormMode::Edit => {
                if let Some(task_id) = form.editing_task_id {
                    let prefix = task_id.to_string();
                    let prefix = &prefix[..8.min(prefix.len())];
                    match self.service.edit_task(
                        prefix,
                        Some(form.title.trim().to_string()),
                        description,
                        Some(priority),
                        label,
                        project,
                        None,
                        Some(recurrence),
                    ) {
                        Ok(_) => {
                            self.set_status_message("Task updated".to_string());
                        }
                        Err(e) => {
                            self.set_status_message(format!("Failed to update task: {e}"));
                        }
                    }
                }
            }
            FormMode::Subtask => {
                let parent_str = form.parent_id.map(|id| {
                    let s = id.to_string();
                    s[..8.min(s.len())].to_string()
                });
                match self.service.add_task(
                    form.title.trim().to_string(),
                    description,
                    priority,
                    CreatedBy::Human,
                    label,
                    project,
                    parent_str,
                    Recurrence::Never,
                ) {
                    Ok(_) => {
                        self.set_status_message("Task created".to_string());
                    }
                    Err(e) => {
                        self.set_status_message(format!("Failed to create subtask: {e}"));
                    }
                }
            }
        }

        self.state = AppState::TaskList;
        self.update_mtime();
        self.service_invalidate_cache();
        self.reload_tasks();
    }

    fn toggle_status(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let prefix = self.task_id_prefix(&task);
            let new_status = Self::next_status(task.status);
            match self.service.change_status(&prefix, new_status) {
                Ok(result) => {
                    if let Some(ref generated) = result.generated_task {
                        self.set_status_message(format!(
                            "繰り返しタスクを生成しました: {}",
                            generated.title
                        ));
                    }
                }
                Err(e) => {
                    self.set_status_message(format!("Status change failed: {e}"));
                }
            }
            self.update_mtime();
            self.service_invalidate_cache();
            self.reload_tasks();
        }
    }

    fn cancel_task(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let prefix = self.task_id_prefix(&task);
            match self.service.change_status(&prefix, "cancelled") {
                Ok(_) => {
                    self.set_status_message(format!("Cancelled '{}'", task.title));
                }
                Err(e) => {
                    self.set_status_message(format!("Cancel failed: {e}"));
                }
            }
            self.update_mtime();
            self.service_invalidate_cache();
            self.reload_tasks();
        }
    }

    fn open_delete_confirm(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            self.delete_target = Some(task);
            self.state = AppState::DeleteConfirm;
        }
    }

    fn get_available_labels(&self) -> Vec<String> {
        // Get labels from settings via the service
        // We'll scan existing tasks for used labels and combine with known builtins
        let all_filter = TaskFilter {
            include_done: true,
            include_cancelled: true,
            ..Default::default()
        };
        let mut labels: Vec<String> = vec![
            "bug".to_string(),
            "feature".to_string(),
            "improvement".to_string(),
            "documentation".to_string(),
            "refactor".to_string(),
            "chore".to_string(),
        ];
        if let Ok(all_tasks) = self.service.list_tasks(&all_filter) {
            for task in &all_tasks {
                if let Some(ref label) = task.label {
                    if !labels.contains(label) {
                        labels.push(label.clone());
                    }
                }
            }
        }
        labels
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

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::model::filter::TaskFilter;
use crate::model::task::{CreatedBy, TaskId};

use super::{App, AppState, FormMode, KeyResult, PRIORITIES, TaskFormData};

impl App {
    pub(super) fn handle_key_form(&mut self, key: KeyEvent) -> KeyResult {
        // Field indices: 0=title, 1=priority, 2=label, 3=project,
        //                4=parent, 5=content
        const FIELD_COUNT: usize = 6;

        if let Some(ref mut form) = self.form {
            match key.code {
                KeyCode::Esc => {
                    self.form = None;
                    self.state = AppState::TaskList;
                }
                KeyCode::Enter => {
                    if form.focused_field == 5 && key.modifiers.contains(KeyModifiers::ALT) {
                        content_insert_newline(form);
                    } else {
                        self.save_form();
                    }
                }
                KeyCode::Tab => {
                    form.focused_field = (form.focused_field + 1) % FIELD_COUNT;
                }
                KeyCode::BackTab => {
                    if form.focused_field == 0 {
                        form.focused_field = FIELD_COUNT - 1;
                    } else {
                        form.focused_field -= 1;
                    }
                }
                KeyCode::Backspace => match form.focused_field {
                    0 => text_backspace(&mut form.title, &mut form.title_cursor),
                    3 => text_backspace(&mut form.project, &mut form.project_cursor),
                    5 => content_backspace(form),
                    _ => {}
                },
                KeyCode::Up => {
                    if form.focused_field == 5 {
                        content_move_up(form);
                    }
                }
                KeyCode::Down => {
                    if form.focused_field == 5 {
                        content_move_down(form);
                    }
                }
                KeyCode::Left => match form.focused_field {
                    0 => text_move_left(&form.title, &mut form.title_cursor),
                    3 => text_move_left(&form.project, &mut form.project_cursor),
                    5 => content_move_left(form),
                    _ => handle_selector_left(form),
                },
                KeyCode::Right => match form.focused_field {
                    0 => text_move_right(&form.title, &mut form.title_cursor),
                    3 => text_move_right(&form.project, &mut form.project_cursor),
                    5 => content_move_right(form),
                    _ => handle_selector_right(form),
                },
                KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    if form.focused_field == 5 {
                        content_insert_newline(form);
                    }
                }
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        // Ignore other ctrl combos
                    } else {
                        match form.focused_field {
                            0 => text_insert_char(&mut form.title, &mut form.title_cursor, c),
                            3 => text_insert_char(&mut form.project, &mut form.project_cursor, c),
                            5 => content_insert_char(form, c),
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        KeyResult { should_quit: false }
    }

    pub(super) fn open_new_task_form(&mut self) {
        let labels = self.get_available_labels();
        let parents = self.get_available_parents(None);
        self.form = Some(TaskFormData {
            mode: FormMode::New,
            title: String::new(),
            title_cursor: 0,
            content: String::new(),
            content_cursor_row: 0,
            content_cursor_col: 0,
            priority_index: 0,
            label_index: 0,
            project: String::new(),
            project_cursor: 0,
            parent_index: 0,
            available_parents: parents,
            editing_task_id: None,
            focused_field: 0,
            available_labels: labels,
        });
        self.state = AppState::TaskForm;
    }

    pub(super) fn open_edit_form(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let labels = self.get_available_labels();
            let parents = self.get_available_parents(Some(task.id));
            let priority_index = PRIORITIES
                .iter()
                .position(|p| *p == task.priority)
                .unwrap_or(0);
            let label_index = match &task.label {
                None => 0,
                Some(l) => labels.iter().position(|lab| lab == l).map_or(0, |i| i + 1),
            };
            let parent_index = match task.parent_id {
                None => 0,
                Some(pid) => parents
                    .iter()
                    .position(|(id, _)| *id == pid)
                    .map_or(0, |i| i + 1),
            };

            let content_text = task.content.clone().unwrap_or_default();
            let c_lines: Vec<&str> = content_text.split('\n').collect();
            let last_row = if c_lines.is_empty() {
                0
            } else {
                c_lines.len() - 1
            };
            let last_col = c_lines.last().map_or(0, |l| l.chars().count());

            let title_len = task.title.chars().count();
            let project_text = task.project.clone().unwrap_or_default();
            let project_len = project_text.chars().count();

            self.form = Some(TaskFormData {
                mode: FormMode::Edit,
                title: task.title.clone(),
                title_cursor: title_len,
                content: content_text,
                content_cursor_row: last_row,
                content_cursor_col: last_col,
                priority_index,
                label_index,
                project: project_text,
                project_cursor: project_len,
                parent_index,
                available_parents: parents,
                editing_task_id: Some(task.id),
                focused_field: 0,
                available_labels: labels,
            });
            self.state = AppState::TaskForm;
        }
    }

    pub(super) fn open_subtask_form(&mut self) {
        if let Some(task) = self.selected_task().cloned() {
            let actual_parent_id = task.parent_id.unwrap_or(task.id);

            let labels = self.get_available_labels();
            let parents = self.get_available_parents(None);
            let parent_index = parents
                .iter()
                .position(|(id, _)| *id == actual_parent_id)
                .map_or(0, |i| i + 1);

            self.form = Some(TaskFormData {
                mode: FormMode::Subtask,
                title: String::new(),
                title_cursor: 0,
                content: String::new(),
                content_cursor_row: 0,
                content_cursor_col: 0,
                priority_index: 0,
                label_index: 0,
                project: task.project.clone().unwrap_or_default(),
                project_cursor: 0,
                parent_index,
                available_parents: parents,
                editing_task_id: None,
                focused_field: 0,
                available_labels: labels,
            });
            self.state = AppState::TaskForm;
        }
    }

    pub(super) fn save_form(&mut self) {
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
        let content = if form.content.trim().is_empty() {
            None
        } else {
            Some(form.content.trim().to_string())
        };

        let parent_str = form.selected_parent_id().map(|id| {
            let s = id.to_string();
            s[..8.min(s.len())].to_string()
        });
        let has_parent = parent_str.is_some();

        let result = match form.mode {
            FormMode::New | FormMode::Subtask => self
                .service
                .add_task(
                    form.title.trim().to_string(),
                    content,
                    priority,
                    CreatedBy::Human,
                    label,
                    project,
                    parent_str,
                )
                .map(|_| {
                    if has_parent {
                        "Subtask created".to_string()
                    } else {
                        "Task created".to_string()
                    }
                })
                .map_err(|e| format!("Failed to create task: {e}")),
            FormMode::Edit => {
                if let Some(task_id) = form.editing_task_id {
                    let prefix = task_id.to_string();
                    let prefix_str = &prefix[..8.min(prefix.len())];
                    let parent_param = match form.selected_parent_id() {
                        Some(pid) => {
                            let s = pid.to_string();
                            Some(s[..8.min(s.len())].to_string())
                        }
                        None => Some("none".to_string()),
                    };
                    self.service
                        .edit_task(
                            prefix_str,
                            Some(form.title.trim().to_string()),
                            content,
                            Some(priority),
                            label,
                            project,
                            parent_param,
                        )
                        .map(|_| "Task updated".to_string())
                        .map_err(|e| format!("Failed to update task: {e}"))
                } else {
                    Err("No task ID for edit".to_string())
                }
            }
        };

        match result {
            Ok(msg) => {
                self.set_status_message(msg);
                self.state = AppState::TaskList;
                self.update_mtime();
                self.reload_tasks();
            }
            Err(msg) => {
                self.set_status_message(msg);
                self.form = Some(form);
                self.state = AppState::TaskForm;
            }
        }
    }

    pub(super) fn get_available_parents(
        &self,
        exclude_id: Option<TaskId>,
    ) -> Vec<(TaskId, String)> {
        let all_filter = TaskFilter {
            include_done: false,
            include_cancelled: false,
            ..Default::default()
        };
        let mut parents = Vec::new();
        if let Ok(all_tasks) = self.service.list_tasks(&all_filter) {
            for task in &all_tasks {
                if task.parent_id.is_some() {
                    continue;
                }
                if exclude_id.is_some_and(|eid| eid == task.id) {
                    continue;
                }
                parents.push((task.id, task.title.clone()));
            }
        }
        parents.sort_by(|a, b| a.1.cmp(&b.1));
        parents
    }

    pub(super) fn get_available_labels(&self) -> Vec<String> {
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
                if let Some(ref label) = task.label
                    && !labels.contains(label)
                {
                    labels.push(label.clone());
                }
            }
        }
        labels
    }
}

// --- Single-line text field helpers ---

fn text_insert_char(text: &mut String, cursor: &mut usize, c: char) {
    let char_count = text.chars().count();
    if *cursor > char_count {
        *cursor = char_count;
    }
    let byte_idx: usize = text.chars().take(*cursor).map(|ch| ch.len_utf8()).sum();
    text.insert(byte_idx, c);
    *cursor += 1;
}

fn text_backspace(text: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let char_count = text.chars().count();
    if *cursor > char_count {
        *cursor = char_count;
    }
    let byte_start: usize = text.chars().take(*cursor - 1).map(|ch| ch.len_utf8()).sum();
    let byte_end: usize = text.chars().take(*cursor).map(|ch| ch.len_utf8()).sum();
    text.drain(byte_start..byte_end);
    *cursor -= 1;
}

fn text_move_left(_text: &str, cursor: &mut usize) {
    if *cursor > 0 {
        *cursor -= 1;
    }
}

fn text_move_right(text: &str, cursor: &mut usize) {
    let char_count = text.chars().count();
    if *cursor < char_count {
        *cursor += 1;
    }
}

// --- Multiline content helpers ---

fn content_lines(content: &str) -> Vec<String> {
    content.split('\n').map(|s| s.to_string()).collect()
}

fn content_from_lines(lines: &[String]) -> String {
    lines.join("\n")
}

fn content_insert_char(form: &mut TaskFormData, c: char) {
    let mut lines = content_lines(&form.content);
    if lines.is_empty() {
        lines.push(String::new());
    }
    if form.content_cursor_row >= lines.len() {
        form.content_cursor_row = lines.len() - 1;
    }
    let line = &mut lines[form.content_cursor_row];
    let char_count = line.chars().count();
    if form.content_cursor_col > char_count {
        form.content_cursor_col = char_count;
    }
    let byte_idx: usize = line
        .chars()
        .take(form.content_cursor_col)
        .map(|ch| ch.len_utf8())
        .sum();
    line.insert(byte_idx, c);
    form.content_cursor_col += 1;
    form.content = content_from_lines(&lines);
}

fn content_insert_newline(form: &mut TaskFormData) {
    let mut lines = content_lines(&form.content);
    if lines.is_empty() {
        lines.push(String::new());
    }
    if form.content_cursor_row >= lines.len() {
        form.content_cursor_row = lines.len() - 1;
    }
    let line = &lines[form.content_cursor_row];
    let char_count = line.chars().count();
    if form.content_cursor_col > char_count {
        form.content_cursor_col = char_count;
    }
    let byte_idx: usize = line
        .chars()
        .take(form.content_cursor_col)
        .map(|ch| ch.len_utf8())
        .sum();
    let remainder = line[byte_idx..].to_string();
    let current = line[..byte_idx].to_string();
    lines[form.content_cursor_row] = current;
    lines.insert(form.content_cursor_row + 1, remainder);
    form.content_cursor_row += 1;
    form.content_cursor_col = 0;
    form.content = content_from_lines(&lines);
}

fn content_backspace(form: &mut TaskFormData) {
    let mut lines = content_lines(&form.content);
    if lines.is_empty() {
        return;
    }
    if form.content_cursor_row >= lines.len() {
        form.content_cursor_row = lines.len() - 1;
    }
    let line = &lines[form.content_cursor_row];
    let char_count = line.chars().count();
    if form.content_cursor_col > char_count {
        form.content_cursor_col = char_count;
    }

    if form.content_cursor_col > 0 {
        let byte_start: usize = line
            .chars()
            .take(form.content_cursor_col - 1)
            .map(|ch| ch.len_utf8())
            .sum();
        let byte_end: usize = line
            .chars()
            .take(form.content_cursor_col)
            .map(|ch| ch.len_utf8())
            .sum();
        let mut new_line = line[..byte_start].to_string();
        new_line.push_str(&line[byte_end..]);
        lines[form.content_cursor_row] = new_line;
        form.content_cursor_col -= 1;
    } else if form.content_cursor_row > 0 {
        let current_line = lines.remove(form.content_cursor_row);
        form.content_cursor_row -= 1;
        let prev_char_count = lines[form.content_cursor_row].chars().count();
        lines[form.content_cursor_row].push_str(&current_line);
        form.content_cursor_col = prev_char_count;
    }
    form.content = content_from_lines(&lines);
}

fn content_move_up(form: &mut TaskFormData) {
    if form.content_cursor_row > 0 {
        form.content_cursor_row -= 1;
        let lines = content_lines(&form.content);
        let line_chars = lines[form.content_cursor_row].chars().count();
        if form.content_cursor_col > line_chars {
            form.content_cursor_col = line_chars;
        }
    }
}

fn content_move_down(form: &mut TaskFormData) {
    let lines = content_lines(&form.content);
    if form.content_cursor_row + 1 < lines.len() {
        form.content_cursor_row += 1;
        let line_chars = lines[form.content_cursor_row].chars().count();
        if form.content_cursor_col > line_chars {
            form.content_cursor_col = line_chars;
        }
    }
}

fn content_move_left(form: &mut TaskFormData) {
    if form.content_cursor_col > 0 {
        form.content_cursor_col -= 1;
    } else if form.content_cursor_row > 0 {
        // Wrap to end of previous line
        form.content_cursor_row -= 1;
        let lines = content_lines(&form.content);
        form.content_cursor_col = lines[form.content_cursor_row].chars().count();
    }
}

fn content_move_right(form: &mut TaskFormData) {
    let lines = content_lines(&form.content);
    if form.content_cursor_row >= lines.len() {
        return;
    }
    let line_chars = lines[form.content_cursor_row].chars().count();
    if form.content_cursor_col < line_chars {
        form.content_cursor_col += 1;
    } else if form.content_cursor_row + 1 < lines.len() {
        // Wrap to start of next line
        form.content_cursor_row += 1;
        form.content_cursor_col = 0;
    }
}

fn handle_selector_left(form: &mut TaskFormData) {
    match form.focused_field {
        1 => {
            if form.priority_index > 0 {
                form.priority_index -= 1;
            }
        }
        2 => {
            if form.label_index > 0 {
                form.label_index -= 1;
            }
        }
        4 => {
            if form.mode != FormMode::Subtask && form.parent_index > 0 {
                form.parent_index -= 1;
            }
        }
        _ => {}
    }
}

fn handle_selector_right(form: &mut TaskFormData) {
    match form.focused_field {
        1 => {
            if form.priority_index < PRIORITIES.len() - 1 {
                form.priority_index += 1;
            }
        }
        2 => {
            let max = form.available_labels.len();
            if form.label_index < max {
                form.label_index += 1;
            }
        }
        4 => {
            if form.mode != FormMode::Subtask && form.parent_index < form.available_parents.len() {
                form.parent_index += 1;
            }
        }
        _ => {}
    }
}

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::model::recurrence::Recurrence;
use crate::model::task::{CreatedBy, Priority, Status, Task};
use crate::tui::app::App;

/// Render the detail panel for a selected task.
pub fn render(frame: &mut Frame, task: &Task, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Title
    lines.push(Line::from(vec![
        Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&task.title),
    ]));

    // ID (short)
    let id_short = &task.id.to_string()[..8];
    lines.push(Line::from(vec![
        Span::styled("ID: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(id_short, Style::default().fg(Color::DarkGray)),
    ]));

    // Status
    let status_str = task.status.to_string();
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(status_str, status_color(task.status)),
    ]));

    // Priority
    let priority_str = task.priority.to_string();
    lines.push(Line::from(vec![
        Span::styled("Priority: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(priority_str, priority_color(task.priority)),
    ]));

    // Description
    if let Some(ref desc) = task.description {
        lines.push(Line::from(vec![
            Span::styled("Description: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(desc),
        ]));
    }

    // Label
    if let Some(ref label) = task.label {
        lines.push(Line::from(vec![
            Span::styled("Label: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(label),
        ]));
    }

    // Project
    if let Some(ref project) = task.project {
        lines.push(Line::from(vec![
            Span::styled("Project: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(project),
        ]));
    }

    // Created by
    let created_by_str = match task.created_by {
        CreatedBy::Human => "human",
        CreatedBy::Ai => "ai",
    };
    lines.push(Line::from(vec![
        Span::styled("Created by: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(created_by_str),
    ]));

    // Recurrence
    if task.recurrence != Recurrence::Never {
        let rec_str = format!("{:?}", task.recurrence);
        lines.push(Line::from(vec![
            Span::styled(
                "Recurrence: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(rec_str),
        ]));
    }

    // Created at
    lines.push(Line::from(vec![
        Span::styled("Created: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(task.created_at.format("%Y-%m-%d %H:%M").to_string()),
    ]));

    // Updated at
    lines.push(Line::from(vec![
        Span::styled("Updated: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(task.updated_at.format("%Y-%m-%d %H:%M").to_string()),
    ]));

    // Parent info (for subtasks)
    if let Some(parent_id) = task.parent_id {
        let parent_title = find_parent_title(app, parent_id);
        lines.push(Line::from(vec![
            Span::styled("Parent: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(parent_title),
        ]));
    }

    // Subtask count (for parent tasks)
    if task.parent_id.is_none() {
        let subtask_count = count_subtasks(app, task.id);
        if subtask_count > 0 {
            lines.push(Line::from(vec![
                Span::styled(
                    "Subtasks: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(subtask_count.to_string()),
            ]));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Detail");
    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

fn find_parent_title(app: &App, parent_id: uuid::Uuid) -> String {
    // Look in the current task list first
    for task in app.tasks() {
        if task.id == parent_id {
            return task.title.clone();
        }
    }
    // Try the service
    let prefix = &parent_id.to_string()[..8];
    match app.service().get_task(prefix) {
        Ok(parent) => parent.title,
        Err(_) => "Unknown".to_string(),
    }
}

fn count_subtasks(app: &App, parent_id: uuid::Uuid) -> usize {
    match app.service().get_subtasks(parent_id) {
        Ok(children) => children.len(),
        Err(_) => 0,
    }
}

fn status_color(status: Status) -> Style {
    match status {
        Status::Todo => Style::default().fg(Color::White),
        Status::InProgress => Style::default().fg(Color::Yellow),
        Status::Done => Style::default().fg(Color::Green),
        Status::Cancelled => Style::default().fg(Color::DarkGray),
    }
}

fn priority_color(priority: Priority) -> Style {
    match priority {
        Priority::None => Style::default().fg(Color::DarkGray),
        Priority::Low => Style::default().fg(Color::Blue),
        Priority::Medium => Style::default().fg(Color::Yellow),
        Priority::High => Style::default().fg(Color::Red),
        Priority::Critical => Style::default()
            .fg(Color::Red)
            .add_modifier(Modifier::BOLD),
    }
}

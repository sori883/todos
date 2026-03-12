use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::model::task::{CreatedBy, Priority, Status, Task};
use crate::tui::app::App;

/// Render the detail panel for a selected task.
pub fn render(frame: &mut Frame, task: &Task, app: &App, area: Rect) {
    let inner_width = area.width.saturating_sub(2) as usize;
    let mut lines: Vec<Line> = Vec::new();

    // ── Title (prominent) ──
    lines.push(Line::from(Span::styled(
        task.title.clone(),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )));

    // ID (dim)
    let id_str = task.id.to_string();
    let id_short = &id_str[..8.min(id_str.len())];
    lines.push(Line::from(Span::styled(
        format!("#{id_short}"),
        Style::default().fg(Color::DarkGray),
    )));

    lines.push(Line::from(""));

    // ── Status + Priority badges ──
    let mut badge_spans = vec![status_badge(task.status)];
    if let Some(pb) = priority_badge(task.priority) {
        badge_spans.push(Span::raw("  "));
        badge_spans.push(pb);
    }
    lines.push(Line::from(badge_spans));

    // ── Metadata section ──
    lines.push(separator_line(inner_width));

    if let Some(ref label) = task.label {
        push_field(&mut lines, "Label", label, Color::White);
    }
    if let Some(ref project) = task.project {
        push_field(&mut lines, "Project", project, Color::Cyan);
    }

    let created_by_str = match task.created_by {
        CreatedBy::Human => "human",
        CreatedBy::Ai => "ai",
    };
    push_field(&mut lines, "Author", created_by_str, Color::White);

    if let Some(parent_id) = task.parent_id {
        let parent_title = find_parent_title(app, parent_id);
        push_field(&mut lines, "Parent", &parent_title, Color::Cyan);
    }

    if task.parent_id.is_none() {
        let subtask_count = count_subtasks(app, task.id);
        if subtask_count > 0 {
            push_field(&mut lines, "Subtasks", &subtask_count.to_string(), Color::White);
        }
    }

    // ── Content section ──
    if let Some(ref content) = task.content {
        lines.push(separator_line(inner_width));
        for line in content.lines() {
            lines.push(Line::from(Span::raw(line)));
        }
    }

    // ── Timestamps (dim) ──
    lines.push(separator_line(inner_width));
    push_field(
        &mut lines,
        "Created",
        &task.created_at.format("%Y-%m-%d %H:%M").to_string(),
        Color::DarkGray,
    );
    push_field(
        &mut lines,
        "Updated",
        &task.updated_at.format("%Y-%m-%d %H:%M").to_string(),
        Color::DarkGray,
    );

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Detail")
        .border_style(Style::default().fg(Color::DarkGray));
    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

// ── Helpers ──

fn separator_line(width: usize) -> Line<'static> {
    let sep: String = "─".repeat(width);
    Line::from(Span::styled(sep, Style::default().fg(Color::DarkGray)))
}

fn push_field(lines: &mut Vec<Line>, label: &str, value: &str, value_color: Color) {
    let padded = format!("{label:<10}");
    lines.push(Line::from(vec![
        Span::styled(padded, Style::default().fg(Color::DarkGray)),
        Span::styled(value.to_string(), Style::default().fg(value_color)),
    ]));
}

fn status_badge(status: Status) -> Span<'static> {
    match status {
        Status::Todo => Span::styled(
            " ○ todo ",
            Style::default().fg(Color::White),
        ),
        Status::InProgress => Span::styled(
            " ● in_progress ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Status::Done => Span::styled(
            " ✓ done ",
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Status::Cancelled => Span::styled(
            " ✗ cancelled ",
            Style::default().fg(Color::White).bg(Color::DarkGray),
        ),
    }
}

fn priority_badge(priority: Priority) -> Option<Span<'static>> {
    match priority {
        Priority::None => None,
        Priority::Low => Some(Span::styled(
            " low ",
            Style::default().fg(Color::Blue),
        )),
        Priority::Medium => Some(Span::styled(
            " medium ",
            Style::default().fg(Color::Black).bg(Color::Yellow),
        )),
        Priority::High => Some(Span::styled(
            " high ",
            Style::default().fg(Color::White).bg(Color::Red),
        )),
        Priority::Critical => Some(Span::styled(
            " critical ",
            Style::default()
                .fg(Color::White)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD),
        )),
    }
}

fn find_parent_title(app: &App, parent_id: uuid::Uuid) -> String {
    for task in app.tasks() {
        if task.id == parent_id {
            return task.title.clone();
        }
    }
    let id_str = parent_id.to_string();
    let prefix = &id_str[..8.min(id_str.len())];
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

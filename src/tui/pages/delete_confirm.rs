use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::model::task::Task;

/// Render a delete confirmation dialog as a centered overlay.
pub fn render(frame: &mut Frame, task: &Task, subtask_count: usize) {
    let area = frame.area();

    let dialog_width = 50u16.min(area.width.saturating_sub(4));
    let dialog_height = if subtask_count > 0 { 7 } else { 5 };
    let dialog_height = dialog_height.min(area.height.saturating_sub(2));
    let dialog_area = centered_rect(dialog_width, dialog_height, area);

    frame.render_widget(Clear, dialog_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Delete Confirm")
        .style(Style::default().bg(Color::Black).fg(Color::Red));

    let inner = block.inner(dialog_area);
    frame.render_widget(block, dialog_area);

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled(
            format!("Delete '{}'?", task.title),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    if subtask_count > 0 {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![Span::styled(
            format!("This will also delete {subtask_count} subtask(s)"),
            Style::default().fg(Color::Yellow),
        )]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(": yes  "),
        Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw("/"),
        Span::styled("Esc", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Span::raw(": cancel"),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

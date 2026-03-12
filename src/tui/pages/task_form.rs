use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::tui::app::{FormMode, TaskFormData, PRIORITIES, RECURRENCES};

/// Render the task form as a centered overlay.
pub fn render(frame: &mut Frame, form: &TaskFormData) {
    let area = frame.area();

    // Center the form: 60 wide, 16 tall
    let form_width = 60u16.min(area.width.saturating_sub(4));
    let form_height = 16u16.min(area.height.saturating_sub(4));
    let form_area = centered_rect(form_width, form_height, area);

    // Clear the area behind the form
    frame.render_widget(Clear, form_area);

    let title = match form.mode {
        FormMode::New => "New Task",
        FormMode::Edit => "Edit Task",
        FormMode::Subtask => "New Subtask",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(Style::default().bg(Color::Black));

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    // Layout the fields vertically
    let field_constraints: Vec<Constraint> = vec![
        Constraint::Length(2), // title
        Constraint::Length(2), // description
        Constraint::Length(2), // priority
        Constraint::Length(2), // label
        Constraint::Length(2), // project
        Constraint::Length(2), // recurrence
        Constraint::Min(1),   // help text
    ];

    let field_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_constraints)
        .split(inner);

    // Render each field
    render_text_field(frame, "Title", &form.title, form.focused_field == 0, field_areas[0]);
    render_text_field(
        frame,
        "Description",
        &form.description,
        form.focused_field == 1,
        field_areas[1],
    );
    render_selector_field(
        frame,
        "Priority",
        &priority_display(form.priority_index),
        form.focused_field == 2,
        field_areas[2],
    );
    render_selector_field(
        frame,
        "Label",
        &label_display(form.label_index, &form.available_labels),
        form.focused_field == 3,
        field_areas[3],
    );
    render_text_field(
        frame,
        "Project",
        &form.project,
        form.focused_field == 4,
        field_areas[4],
    );

    let recurrence_locked = form.mode == FormMode::Subtask;
    render_selector_field(
        frame,
        if recurrence_locked {
            "Recurrence (locked)"
        } else {
            "Recurrence"
        },
        &recurrence_display(form.recurrence_index),
        form.focused_field == 5 && !recurrence_locked,
        field_areas[5],
    );

    // Help text
    let help = Line::from(vec![
        Span::styled(
            "Tab/Shift-Tab: navigate  Left/Right: select  Enter: save  Esc: cancel",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let help_paragraph = Paragraph::new(help);
    frame.render_widget(help_paragraph, field_areas[6]);
}

fn render_text_field(frame: &mut Frame, label: &str, value: &str, focused: bool, area: Rect) {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let display_value = if focused {
        format!("{value}_")
    } else {
        value.to_string()
    };

    let line = Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(display_value, style),
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

fn render_selector_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    focused: bool,
    area: Rect,
) {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let arrows = if focused { "< " } else { "  " };
    let arrows_end = if focused { " >" } else { "  " };

    let line = Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::styled(arrows, style),
        Span::styled(value, style),
        Span::styled(arrows_end, style),
    ]);

    let paragraph = Paragraph::new(line);
    frame.render_widget(paragraph, area);
}

fn priority_display(index: usize) -> String {
    if index < PRIORITIES.len() {
        PRIORITIES[index].to_string()
    } else {
        "none".to_string()
    }
}

fn label_display(index: usize, labels: &[String]) -> String {
    if index == 0 {
        "(none)".to_string()
    } else if let Some(label) = labels.get(index - 1) {
        label.clone()
    } else {
        "(none)".to_string()
    }
}

fn recurrence_display(index: usize) -> String {
    if index < RECURRENCES.len() {
        RECURRENCES[index].to_string()
    } else {
        "never".to_string()
    }
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

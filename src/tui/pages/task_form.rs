use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use unicode_width::UnicodeWidthChar;

use crate::tui::app::{FormMode, TaskFormData, PRIORITIES};

/// Render the task form as a centered overlay.
pub fn render(frame: &mut Frame, form: &TaskFormData) {
    let area = frame.area();

    // Guard: skip rendering if terminal is too small for the form
    if area.width < super::MIN_WIDTH || area.height < super::MIN_HEIGHT {
        return;
    }

    // Center the form: 90% of terminal area
    let form_width = (area.width * 9 / 10).max(60u16.min(area.width));
    let form_height = area.height * 9 / 10;
    let form_area = super::centered_rect(form_width, form_height, area);

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

    // Layout: fixed fields on top, content fills remaining space, help at bottom
    let field_constraints: Vec<Constraint> = vec![
        Constraint::Length(2), // 0: title
        Constraint::Length(2), // 1: priority
        Constraint::Length(2), // 2: label
        Constraint::Length(2), // 3: project
        Constraint::Length(2), // 4: parent
        Constraint::Min(3),   // 5: content (fills remaining space)
        Constraint::Length(1), // help text
    ];

    let field_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(field_constraints)
        .split(inner);

    // Render each field
    render_text_field(frame, "Title", &form.title, form.title_cursor, form.focused_field == 0, field_areas[0]);
    render_selector_field(
        frame,
        "Priority",
        &priority_display(form.priority_index),
        form.focused_field == 1,
        field_areas[1],
    );
    render_selector_field(
        frame,
        "Label",
        &label_display(form.label_index, &form.available_labels),
        form.focused_field == 2,
        field_areas[2],
    );
    render_text_field(
        frame,
        "Project",
        &form.project,
        form.project_cursor,
        form.focused_field == 3,
        field_areas[3],
    );

    // Parent selector
    let parent_locked = form.mode == FormMode::Subtask;
    render_selector_field(
        frame,
        if parent_locked {
            "Parent (locked)"
        } else {
            "Parent"
        },
        &parent_display(form.parent_index, &form.available_parents),
        form.focused_field == 4 && !parent_locked,
        field_areas[4],
    );

    // Content (multiline, fills remaining space)
    render_multiline_field(
        frame,
        "Content",
        &form.content,
        form.content_cursor_row,
        form.content_cursor_col,
        form.focused_field == 5,
        field_areas[5],
    );

    // Help text
    let help = Line::from(vec![
        Span::styled(
            "Tab/S-Tab: navigate  Left/Right: select  Enter: save  Esc: cancel  Alt+Enter: newline",
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    let help_paragraph = Paragraph::new(help);
    frame.render_widget(help_paragraph, field_areas[6]);
}

fn render_text_field(frame: &mut Frame, label: &str, value: &str, cursor_pos: usize, focused: bool, area: Rect) {
    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let mut spans = vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];

    if focused {
        spans.extend(build_cursor_spans(value, cursor_pos, style));
    } else {
        spans.push(Span::styled(value.to_string(), style));
    }

    let paragraph = Paragraph::new(Line::from(spans));
    frame.render_widget(paragraph, area);
}

/// Wrap a single logical line into visual lines based on display width.
/// Returns a vec of (start_char_index, end_char_index) for each visual line.
fn wrap_line_ranges(line: &str, max_width: usize) -> Vec<(usize, usize)> {
    if max_width == 0 {
        return vec![(0, line.chars().count())];
    }
    let chars: Vec<char> = line.chars().collect();
    if chars.is_empty() {
        return vec![(0, 0)];
    }

    let mut ranges = Vec::new();
    let mut start = 0;
    let mut col_width = 0;

    for (i, ch) in chars.iter().enumerate() {
        let w = ch.width().unwrap_or(0);
        if col_width + w > max_width && start < i {
            ranges.push((start, i));
            start = i;
            col_width = w;
        } else {
            col_width += w;
        }
    }
    ranges.push((start, chars.len()));
    ranges
}

fn render_multiline_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    cursor_row: usize,
    cursor_col: usize,
    focused: bool,
    area: Rect,
) {
    if area.height < 2 {
        return;
    }

    let style = if focused {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let content_lines_vec: Vec<&str> = value.split('\n').collect();
    let total_lines = content_lines_vec.len().max(1);
    let max_width = area.width as usize;

    // Label line (1 row) with line indicator when focused
    let label_area = Rect { height: 1, ..area };
    let content_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    let mut label_spans = vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ];
    if focused {
        label_spans.push(Span::styled(
            format!("[{}/{}]", cursor_row + 1, total_lines),
            Style::default().fg(Color::DarkGray),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(label_spans)), label_area);

    // Build visual lines with wrapping
    let mut visual_lines: Vec<Line> = Vec::new();
    let mut cursor_visual_row: usize = 0;

    if value.is_empty() && focused {
        let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
        visual_lines.push(Line::from(Span::styled(" ", cursor_style)));
    } else {
        for (logical_row, line_content) in content_lines_vec.iter().enumerate() {
            let chars: Vec<char> = line_content.chars().collect();
            let ranges = wrap_line_ranges(line_content, max_width);

            for (vi, &(start, end)) in ranges.iter().enumerate() {
                let is_cursor_line = focused
                    && logical_row == cursor_row
                    && cursor_col >= start
                    && (cursor_col < end || (vi == ranges.len() - 1 && cursor_col >= start));

                if is_cursor_line {
                    cursor_visual_row = visual_lines.len();
                    let local_col = cursor_col - start;
                    let segment: String = chars[start..end].iter().collect();
                    visual_lines.push(Line::from(build_cursor_spans(&segment, local_col, style)));
                } else {
                    let segment: String = chars[start..end].iter().collect();
                    visual_lines.push(Line::from(Span::styled(segment, style)));
                }
            }
        }
    }

    let visible_height = content_area.height as usize;
    let scroll_offset = cursor_visual_row.saturating_sub(visible_height.saturating_sub(1));

    let paragraph = Paragraph::new(visual_lines)
        .scroll((scroll_offset as u16, 0));
    frame.render_widget(paragraph, content_area);
}

/// Build spans with a block cursor (white bg, black fg) at the given column position.
fn build_cursor_spans(text: &str, cursor_col: usize, base_style: Style) -> Vec<Span<'static>> {
    let cursor_style = Style::default().bg(Color::White).fg(Color::Black);
    let chars: Vec<char> = text.chars().collect();
    let col = cursor_col.min(chars.len());

    let before: String = chars[..col].iter().collect();
    let cursor_char: String = if col < chars.len() {
        chars[col].to_string()
    } else {
        " ".to_string()
    };
    let after: String = if col < chars.len() {
        chars[col + 1..].iter().collect()
    } else {
        String::new()
    };

    let mut spans = Vec::new();
    if !before.is_empty() {
        spans.push(Span::styled(before, base_style));
    }
    spans.push(Span::styled(cursor_char, cursor_style));
    if !after.is_empty() {
        spans.push(Span::styled(after, base_style));
    }
    spans
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

fn parent_display(index: usize, parents: &[(uuid::Uuid, String)]) -> String {
    if index == 0 {
        "(none)".to_string()
    } else if let Some((_, title)) = parents.get(index - 1) {
        title.clone()
    } else {
        "(none)".to_string()
    }
}

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Tabs};

use crate::model::task::{CreatedBy, Priority, Status};
use crate::tui::app::App;

use super::task_detail;

/// Render the task list page.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Layout: tabs at top, main content, status bar at bottom
    let outer_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // tabs
            Constraint::Min(1),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    // Render project tabs
    render_tabs(frame, app, outer_layout[0]);

    // Main content: list + optional detail panel
    let content_area = outer_layout[1];
    if content_area.width >= 100 {
        // Split into list and detail
        let main_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50),
                Constraint::Percentage(50),
            ])
            .split(content_area);

        render_task_list(frame, app, main_layout[0]);
        render_detail_panel(frame, app, main_layout[1]);
    } else {
        render_task_list(frame, app, content_area);
    }

    // Status bar
    render_status_bar(frame, app, outer_layout[2]);
}

fn render_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let tab_titles: Vec<Line> = app
        .project_tabs()
        .iter()
        .map(|tab| {
            let title = match tab {
                None => "All".to_string(),
                Some(name) => name.clone(),
            };
            Line::from(title)
        })
        .collect();

    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::ALL).title("Projects"))
        .select(app.current_tab())
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_widget(tabs, area);
}

fn render_task_list(frame: &mut Frame, app: &App, area: Rect) {
    let tasks = app.tasks();

    let items: Vec<ListItem> = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let is_selected = i == app.selected_index();
            let is_subtask = task.parent_id.is_some();

            let mut spans: Vec<Span> = Vec::new();

            // Indent for subtasks
            if is_subtask {
                spans.push(Span::raw("  "));
            }

            // Status icon
            let status_icon = match task.status {
                Status::Todo => "[ ] ",
                Status::InProgress => "[>] ",
                Status::Done => "[x] ",
                Status::Cancelled => "[-] ",
            };
            spans.push(Span::styled(
                status_icon,
                status_style(task.status),
            ));

            // Priority badge
            let priority_str = match task.priority {
                Priority::None => "",
                Priority::Low => "[low] ",
                Priority::Medium => "[med] ",
                Priority::High => "[high] ",
                Priority::Critical => "[crit] ",
            };
            if !priority_str.is_empty() {
                spans.push(Span::styled(
                    priority_str,
                    priority_style(task.priority),
                ));
            }

            // Title
            let title_style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else if task.status == Status::Done || task.status == Status::Cancelled {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            spans.push(Span::styled(task.title.clone(), title_style));

            // AI marker
            if task.created_by == CreatedBy::Ai {
                spans.push(Span::styled(
                    " [AI]",
                    Style::default().fg(Color::Cyan),
                ));
            }

            let line = Line::from(spans);
            let style = if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let completed_indicator = if app.show_completed() { " [+done]" } else { "" };
    let title = format!(
        "Tasks ({}){}",
        app.visible_task_count(),
        completed_indicator,
    );

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}

fn render_detail_panel(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(task) = app.selected_task() {
        task_detail::render(frame, task, app, area);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title("Detail");
        let paragraph = Paragraph::new("No task selected")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(paragraph, area);
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let msg = app.status_message();
    let help = if msg.is_empty() {
        "j/k:move  h/l:tab  n:new  e:edit  s:sub  Space:toggle  x:cancel  d:del  c:done  q:quit"
    } else {
        msg
    };

    let bar = Paragraph::new(help)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(bar, area);
}

fn status_style(status: Status) -> Style {
    match status {
        Status::Todo => Style::default().fg(Color::White),
        Status::InProgress => Style::default().fg(Color::Yellow),
        Status::Done => Style::default().fg(Color::Green),
        Status::Cancelled => Style::default().fg(Color::DarkGray),
    }
}

fn priority_style(priority: Priority) -> Style {
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

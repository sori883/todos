pub mod delete_confirm;
pub mod task_detail;
pub mod task_form;
pub mod task_list;

use ratatui::layout::Rect;

/// Minimum terminal width for rendering overlays and full UI.
pub const MIN_WIDTH: u16 = 20;
/// Minimum terminal height for rendering overlays and full UI.
pub const MIN_HEIGHT: u16 = 5;

/// Create a centered Rect within the given area.
/// Width and height are clamped to the area bounds.
pub fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

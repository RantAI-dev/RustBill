use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::app::App;
use crate::ui::widgets::{log_viewer, phase_progress};

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(0)])
        .split(area);

    phase_progress::render(frame, &app.phase_statuses, app.spinner_frame, chunks[0]);
    log_viewer::render(frame, &app.logs, app.log_scroll, chunks[1]);
}

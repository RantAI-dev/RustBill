use ratatui::Frame;
use ratatui::layout::Rect;

use crate::app::App;
use crate::ui::widgets::checklist;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    checklist::render(
        frame,
        "Preflight Checks",
        &app.preflight_checks,
        app.spinner_frame,
        area,
    );
}

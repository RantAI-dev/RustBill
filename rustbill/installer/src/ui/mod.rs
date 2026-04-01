pub mod screens;
pub mod widgets;

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};

use crate::app::{App, Screen};

/// Main render dispatcher — draws the current screen + status bar.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(frame.area());

    match app.screen {
        Screen::Welcome => screens::welcome::render(frame, app, chunks[0]),
        Screen::ModeSelect => screens::mode_select::render(frame, app, chunks[0]),
        Screen::Config => screens::config::render(frame, app, chunks[0]),
        Screen::Preflight => screens::preflight::render(frame, app, chunks[0]),
        Screen::Progress => screens::progress::render(frame, app, chunks[0]),
        Screen::Verify => screens::verify::render(frame, app, chunks[0]),
        Screen::Complete => screens::complete::render(frame, app, chunks[0]),
        Screen::Error => screens::error::render(frame, app, chunks[0]),
    }

    widgets::status_bar::render(frame, app, chunks[1]);
}

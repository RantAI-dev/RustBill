use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, Screen};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let hints = match app.screen {
        Screen::Welcome => "Enter: Continue  q: Quit",
        Screen::ModeSelect => "↑↓: Select  Enter: Continue  Esc: Back  q: Quit",
        Screen::Config => {
            if app.editing {
                "Enter: Save  Esc: Cancel"
            } else {
                "↑↓: Select  e: Edit  Enter: Continue  Esc: Back  q: Quit"
            }
        }
        Screen::Preflight => "Enter: Continue  r: Retry  Esc: Back  q: Quit",
        Screen::Progress => "↑↓: Scroll log",
        Screen::Verify => "Enter: Continue  q: Quit",
        Screen::Complete => "Enter/q: Exit",
        Screen::Error => "Esc: Back  q: Quit",
    };

    let bar = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} Installer ", theme::PRODUCT_NAME),
            theme::style_primary_bold(),
        ),
        Span::styled("│ ", theme::style_muted()),
        Span::styled(hints, theme::style_text_dim()),
    ]));

    frame.render_widget(bar, area);
}

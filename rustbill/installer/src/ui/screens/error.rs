use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Installation Failed", theme::SYM_ERROR),
            theme::style_error().add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(&app.error_message, theme::style_text())),
        Line::from(""),
        Line::from(Span::styled("Troubleshooting:", theme::style_text_dim())),
        Line::from(Span::styled(
            "  • Check the log output above for details",
            theme::style_text(),
        )),
        Line::from(Span::styled(
            "  • Verify system requirements (RAM, disk, ports)",
            theme::style_text(),
        )),
        Line::from(Span::styled(
            "  • Ensure you are running as root or with sudo",
            theme::style_text(),
        )),
        Line::from(Span::styled(
            format!("  • File an issue: https://github.com/{}/issues", theme::REPO),
            theme::style_text(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to go back or q to quit.",
            theme::style_text_dim(),
        )),
    ];

    let block = Block::default()
        .title(" Error ")
        .title_style(theme::style_error().add_modifier(ratatui::style::Modifier::BOLD))
        .borders(Borders::ALL)
        .border_style(theme::style_error());

    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        area,
    );
}

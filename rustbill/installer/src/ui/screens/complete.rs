use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("{} Installation Complete!", theme::SYM_SUCCESS),
            theme::style_success().add_modifier(ratatui::style::Modifier::BOLD),
        )),
        Line::from(""),
    ];

    if app.mode.needs_backend() {
        lines.push(Line::from(vec![
            Span::styled("  API Server:  ", theme::style_text_dim()),
            Span::styled(
                format!("http://localhost:{}", app.config.api_port),
                theme::style_primary(),
            ),
        ]));
        lines.push(Line::from(vec![
            Span::styled("  Health:      ", theme::style_text_dim()),
            Span::styled(
                format!("http://localhost:{}/health", app.config.api_port),
                theme::style_text(),
            ),
        ]));
    }

    if app.mode.needs_frontend() {
        lines.push(Line::from(vec![
            Span::styled("  Dashboard:   ", theme::style_text_dim()),
            Span::styled(
                format!("http://localhost:{}", app.config.frontend_port),
                theme::style_primary(),
            ),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Admin Login: ", theme::style_text_dim()),
        Span::styled(&app.config.admin_email, theme::style_text()),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Service Management:",
        theme::style_text_dim(),
    )));

    if app.mode.needs_services() {
        if app.mode.needs_backend() {
            lines.push(Line::from(Span::styled(
                "  sudo systemctl status rustbill-backend",
                theme::style_text(),
            )));
        }
        if app.mode.needs_frontend() {
            lines.push(Line::from(Span::styled(
                "  sudo systemctl status rustbill-frontend",
                theme::style_text(),
            )));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  sudo journalctl -u rustbill-backend -f   # View logs",
            theme::style_muted(),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Enter or q to exit.",
        theme::style_text_dim(),
    )));

    let block = Block::default()
        .title(format!(" {} ", theme::PRODUCT_NAME))
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    frame.render_widget(
        Paragraph::new(lines).block(block).alignment(Alignment::Center),
        area,
    );
}

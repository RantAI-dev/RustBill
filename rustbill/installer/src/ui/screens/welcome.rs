use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, _app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    // Logo
    for line in theme::LOGO.lines() {
        lines.push(Line::from(Span::styled(line, theme::style_primary_bold())));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Open-Source Billing, Subscription & License Management",
        theme::style_text(),
    )));
    lines.push(Line::from(Span::styled(
        format!("by {}", theme::COMPANY),
        theme::style_text_dim(),
    )));
    lines.push(Line::from(""));
    lines.push(Line::from(""));

    let features = [
        "Subscription lifecycle with proration & plan changes",
        "Multi-provider payments (Stripe, Xendit, LemonSqueezy)",
        "Invoice generation with PDF export",
        "License key management & verification",
        "Usage-based billing with tiered pricing",
        "Automated dunning & payment recovery",
        "Webhooks, analytics & credit system",
    ];

    for feat in features {
        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", theme::SYM_BULLET), theme::style_primary()),
            Span::styled(feat, theme::style_text()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press Enter to begin installation",
        theme::style_text_dim(),
    )));

    let block = Block::default()
        .title(format!(" {} Installer ", theme::PRODUCT_NAME))
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    let widget = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Center);

    frame.render_widget(widget, area);
}

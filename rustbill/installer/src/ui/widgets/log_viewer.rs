use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{LogEntry, LogLevel};
use crate::theme;

pub fn render(frame: &mut Frame, logs: &[LogEntry], scroll: usize, area: Rect) {
    let lines: Vec<Line> = logs
        .iter()
        .map(|entry| {
            let level_style = match entry.level {
                LogLevel::Info => theme::style_text_dim(),
                LogLevel::Success => theme::style_success(),
                LogLevel::Warning => theme::style_warning(),
                LogLevel::Error => theme::style_error(),
            };
            let prefix = match entry.level {
                LogLevel::Info => "INFO",
                LogLevel::Success => " OK ",
                LogLevel::Warning => "WARN",
                LogLevel::Error => " ERR",
            };
            Line::from(vec![
                Span::styled(format!(" {} ", entry.timestamp), theme::style_muted()),
                Span::styled(format!("[{}] ", prefix), level_style),
                Span::styled(&entry.message, theme::style_text()),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Log ")
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    let widget = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll as u16, 0));

    frame.render_widget(widget, area);
}

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::CheckItem;
use crate::theme;

pub fn render(frame: &mut Frame, title: &str, items: &[CheckItem], spinner_frame: usize, area: Rect) {
    let lines: Vec<Line> = items
        .iter()
        .map(|item| {
            let sym = match item.status {
                crate::app::Status::InProgress => theme::SPINNER[spinner_frame],
                _ => theme::status_symbol(item.status),
            };
            let style = theme::status_style(item.status);
            Line::from(vec![
                Span::styled(format!(" {} ", sym), style),
                Span::styled(&item.name, theme::style_text()),
                if item.message.is_empty() {
                    Span::raw("")
                } else {
                    Span::styled(format!("  {}", item.message), theme::style_text_dim())
                },
            ])
        })
        .collect();

    let block = Block::default()
        .title(format!(" {} ", title))
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    let widget = Paragraph::new(lines).block(block);
    frame.render_widget(widget, area);
}

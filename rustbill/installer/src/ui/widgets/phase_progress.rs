use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{Phase, Status};
use crate::theme;

pub fn render(frame: &mut Frame, phases: &[(Phase, Status)], spinner_frame: usize, area: Rect) {
    let lines: Vec<Line> = phases
        .iter()
        .map(|(phase, status)| {
            let sym = match status {
                Status::InProgress => theme::SPINNER[spinner_frame],
                _ => theme::status_symbol(*status),
            };
            let style = theme::status_style(*status);
            let name_style = if *status == Status::InProgress {
                theme::style_text().add_modifier(ratatui::style::Modifier::BOLD)
            } else if *status == Status::Pending {
                theme::style_muted()
            } else {
                theme::style_text()
            };
            Line::from(vec![
                Span::styled(format!(" {} ", sym), style),
                Span::styled(format!("{}. {}", phase.number(), phase.name()), name_style),
            ])
        })
        .collect();

    let block = Block::default()
        .title(" Installation Progress ")
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    let widget = Paragraph::new(lines).block(block);
    frame.render_widget(widget, area);
}

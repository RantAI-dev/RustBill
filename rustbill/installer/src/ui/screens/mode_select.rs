use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, InstallMode};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left: mode list
    let modes = InstallMode::ALL;
    let lines: Vec<Line> = modes
        .iter()
        .enumerate()
        .map(|(i, mode)| {
            let selected = i == app.mode_index;
            let prefix = if selected { theme::SYM_ARROW } else { " " };
            let style = if selected {
                theme::style_highlight()
            } else {
                theme::style_text()
            };
            Line::from(Span::styled(
                format!(" {} {} ", prefix, mode.name()),
                style,
            ))
        })
        .collect();

    let left_block = Block::default()
        .title(" Installation Mode ")
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    frame.render_widget(Paragraph::new(lines).block(left_block), chunks[0]);

    // Right: description of selected mode
    let selected_mode = modes[app.mode_index];
    let mut desc_lines = vec![
        Line::from(Span::styled(
            selected_mode.name(),
            theme::style_primary_bold(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            selected_mode.description(),
            theme::style_text(),
        )),
        Line::from(""),
        Line::from(Span::styled("Components:", theme::style_text_dim())),
    ];

    let components = [
        ("PostgreSQL", selected_mode.needs_database()),
        ("Rust Backend", selected_mode.needs_backend()),
        ("Next.js Frontend", selected_mode.needs_frontend()),
        ("Systemd Services", selected_mode.needs_services()),
    ];

    for (name, included) in components {
        let (sym, style) = if included {
            (theme::SYM_SUCCESS, theme::style_success())
        } else {
            (theme::SYM_SKIPPED, theme::style_muted())
        };
        desc_lines.push(Line::from(vec![
            Span::styled(format!("  {} ", sym), style),
            Span::styled(name, if included { theme::style_text() } else { theme::style_muted() }),
        ]));
    }

    let right_block = Block::default()
        .title(" Details ")
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    frame.render_widget(
        Paragraph::new(desc_lines).block(right_block).wrap(Wrap { trim: false }),
        chunks[1],
    );
}

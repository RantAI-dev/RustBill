use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::App;
use crate::theme;

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(Span::styled(
        " Configure installation parameters. Press 'e' to edit a field.",
        theme::style_text_dim(),
    )));
    lines.push(Line::from(""));

    for (i, field) in app.config_fields.iter().enumerate() {
        let selected = i == app.config_index;
        let is_editing = selected && app.editing;

        let prefix = if selected { theme::SYM_ARROW } else { " " };
        let label_style = if selected {
            theme::style_primary_bold()
        } else {
            theme::style_text()
        };

        let display_value = if is_editing {
            format!("{}▏", app.edit_buffer)
        } else if field.is_secret && !field.value.is_empty() {
            let visible = if field.value.len() > 4 {
                &field.value[..4]
            } else {
                &field.value
            };
            format!("{}•••", visible)
        } else {
            field.value.clone()
        };

        let value_style = if is_editing {
            theme::style_info()
        } else if selected {
            theme::style_text()
        } else {
            theme::style_text_dim()
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", prefix),
                if selected { theme::style_primary() } else { theme::style_muted() },
            ),
            Span::styled(format!("{:<18}", field.label), label_style),
            Span::styled(display_value, value_style),
        ]));
    }

    let block = Block::default()
        .title(format!(" Configuration — {} ", app.mode.name()))
        .title_style(theme::style_primary_bold())
        .borders(Borders::ALL)
        .border_style(theme::style_border());

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

#![allow(dead_code)]
use ratatui::style::{Color, Modifier, Style};

// Brand colors matching RustBill's design system
pub const PRIMARY: Color = Color::Rgb(16, 185, 129); // Emerald #10B981
pub const PRIMARY_DIM: Color = Color::Rgb(6, 95, 70); // Dark emerald
pub const SUCCESS: Color = Color::Rgb(34, 197, 94); // Green
pub const WARNING: Color = Color::Rgb(245, 158, 11); // Amber
pub const ERROR: Color = Color::Rgb(239, 68, 68); // Red
pub const INFO: Color = Color::Rgb(59, 130, 246); // Blue
pub const MUTED: Color = Color::Rgb(107, 114, 128); // Gray-500
pub const TEXT: Color = Color::Rgb(243, 244, 246); // Gray-100
pub const TEXT_DIM: Color = Color::Rgb(156, 163, 175); // Gray-400
pub const BG: Color = Color::Rgb(17, 24, 39); // Gray-900
pub const BG_SURFACE: Color = Color::Rgb(31, 41, 55); // Gray-800
pub const BORDER: Color = Color::Rgb(55, 65, 81); // Gray-700

// Status symbols
pub const SYM_SUCCESS: &str = "✓";
pub const SYM_ERROR: &str = "✗";
pub const SYM_PENDING: &str = "○";
pub const SYM_PROGRESS: &str = "◐";
pub const SYM_WARNING: &str = "⚠";
pub const SYM_SKIPPED: &str = "⊘";
pub const SYM_ARROW: &str = "▶";
pub const SYM_BULLET: &str = "•";

// Spinner frames (braille animation)
pub const SPINNER: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub const PRODUCT_NAME: &str = "RustBill";
pub const COMPANY: &str = "RantAI";
pub const REPO: &str = "RantAI-dev/RustBill";

pub const LOGO: &str = r#"
  ____            _   ____  _ _ _
 |  _ \ _   _ ___| |_| __ )(_) | |
 | |_) | | | / __| __|  _ \| | | |
 |  _ <| |_| \__ \ |_| |_) | | | |
 |_| \_\\__,_|___/\__|____/|_|_|_|
"#;

// Style helpers
pub fn style_primary() -> Style {
    Style::default().fg(PRIMARY)
}

pub fn style_primary_bold() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

pub fn style_success() -> Style {
    Style::default().fg(SUCCESS)
}

pub fn style_warning() -> Style {
    Style::default().fg(WARNING)
}

pub fn style_error() -> Style {
    Style::default().fg(ERROR)
}

pub fn style_info() -> Style {
    Style::default().fg(INFO)
}

pub fn style_muted() -> Style {
    Style::default().fg(MUTED)
}

pub fn style_text() -> Style {
    Style::default().fg(TEXT)
}

pub fn style_text_dim() -> Style {
    Style::default().fg(TEXT_DIM)
}

pub fn style_highlight() -> Style {
    Style::default().bg(PRIMARY_DIM).fg(TEXT).add_modifier(Modifier::BOLD)
}

pub fn style_border() -> Style {
    Style::default().fg(BORDER)
}

use crate::app::Status;

pub fn status_symbol(status: Status) -> &'static str {
    match status {
        Status::Pending => SYM_PENDING,
        Status::InProgress => SYM_PROGRESS,
        Status::Success => SYM_SUCCESS,
        Status::Warning => SYM_WARNING,
        Status::Error => SYM_ERROR,
        Status::Skipped => SYM_SKIPPED,
    }
}

pub fn status_style(status: Status) -> Style {
    match status {
        Status::Pending => style_muted(),
        Status::InProgress => style_info(),
        Status::Success => style_success(),
        Status::Warning => style_warning(),
        Status::Error => style_error(),
        Status::Skipped => style_muted(),
    }
}

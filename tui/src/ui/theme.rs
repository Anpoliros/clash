//! 深色主题：提供清晰边框、明显高亮和低干扰状态色。

use ratatui::style::{Color, Style};

#[derive(Clone, Copy)]
pub struct Theme {
    pub text: Style,
    pub muted: Style,
    pub header: Style,
    pub hover: Style,
    pub active: Style,
    pub action: Style,
    pub status: Style,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            text: Style::default().fg(Color::Gray).bg(Color::Black),
            muted: Style::default().fg(Color::DarkGray).bg(Color::Black),
            header: Style::default().fg(Color::Cyan).bg(Color::Black),
            hover: Style::default().fg(Color::Black).bg(Color::Yellow),
            active: Style::default().fg(Color::Green).bg(Color::Black),
            action: Style::default().fg(Color::LightBlue).bg(Color::Black),
            status: Style::default().fg(Color::Black).bg(Color::Cyan),
        }
    }
}

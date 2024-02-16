use std::cmp::{max, min};

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use unicode_width::UnicodeWidthChar;

use crate::app::{App, Mode};

pub struct SearchWidget {
    pub input: String,
    pub focused: bool,
    pub cursor: usize,
}

impl Default for SearchWidget {
    fn default() -> Self {
        SearchWidget {
            input: "".to_owned(),
            focused: false,
            cursor: 0,
        }
    }
}

impl super::Widget for SearchWidget {
    fn draw(&self, f: &mut Frame, app: &App, area: Rect) {
        let width = self.input.len();
        let fwidth = f.size().width as usize - 2;
        // Try to insert ellipsis if input is too long (visual only)
        let visible = if width >= fwidth {
            let idx = width - fwidth + 2;
            match self.input.get(idx..) {
                Some(sub) => format!("…{}", sub),
                None => self.input.to_owned(),
            }
        } else {
            self.input.to_owned()
        };
        let p = Paragraph::new(visible).block(
            Block::new()
                .borders(Borders::ALL)
                .border_type(app.theme.border)
                .border_style(Style::new().fg(match app.mode {
                    Mode::Search => app.theme.border_focused_color,
                    _ => app.theme.border_color,
                }))
                .fg(app.theme.fg)
                .bg(app.theme.bg)
                .title("Search"),
        );
        f.render_widget(Clear, area);
        f.render_widget(p, area);

        let text = Paragraph::new(Line::from(vec![
            Span::raw("Press "),
            Span::styled("F1", Style::new().bold()),
            Span::raw(" or "),
            Span::styled("?", Style::new().bold()),
            Span::raw(" for help"),
        ]));
        let right = Rect::new(area.right() - 23, area.top(), 23, 1);
        f.render_widget(text, right);
        match app.mode {
            Mode::Search => {
                // Render cursor if in editing mode
                f.set_cursor(
                    min(area.x + self.cursor as u16 + 1, area.x + area.width - 2),
                    area.y + 1,
                );
            }
            _ => {}
        }
    }

    fn handle_event(&mut self, app: &mut crate::app::App, evt: &Event) {
        if let Event::Key(KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers,
            ..
        }) = evt
        {
            use KeyCode::*;
            match (code, modifiers) {
                (Esc, &KeyModifiers::NONE) => {
                    app.mode = Mode::Normal;
                }
                (Char(c), &KeyModifiers::NONE | &KeyModifiers::SHIFT) => {
                    self.input.insert(self.cursor, *c);
                    self.cursor += c.width_cjk().unwrap_or(0);
                }
                (Char('b') | Left, &KeyModifiers::CONTROL) => {
                    // self.cursor = self.input[..self.cursor]
                    //     .rfind(|item| item == ' ')
                    //     .unwrap_or(0);
                    let non_space = self.input[..min(self.cursor, self.input.len())]
                        .rfind(|item| item != ' ')
                        .unwrap_or(0);
                    self.cursor = match self.input[..non_space].rfind(|item| item == ' ') {
                        Some(pos) => pos + 1,
                        None => 0,
                    };
                }
                (Char('w') | Right, &KeyModifiers::CONTROL) => {
                    let idx = min(self.cursor + 1, self.input.len());
                    self.cursor = match self.input[idx..].find(|item| item == ' ') {
                        Some(pos) => self.cursor + pos + 2,
                        None => self.input.len(),
                    };
                }
                (Delete, &KeyModifiers::CONTROL | &KeyModifiers::ALT) => {
                    let idx = min(self.cursor + 1, self.input.len());
                    let new_cursor = match self.input[idx..].find(|item| item == ' ') {
                        Some(pos) => self.cursor + pos + 2,
                        None => self.input.len(),
                    };
                    self.input.replace_range(self.cursor..new_cursor, "");
                }
                (Backspace, &KeyModifiers::ALT | &KeyModifiers::CONTROL) => {
                    let non_space = self.input[..min(self.cursor, self.input.len())]
                        .rfind(|item| item != ' ')
                        .unwrap_or(0);
                    let prev_cursor = self.cursor;
                    self.cursor = match self.input[..non_space].rfind(|item| item == ' ') {
                        Some(pos) => pos + 1,
                        None => 0,
                    };
                    self.input.replace_range(self.cursor..prev_cursor, "");
                }
                (Backspace, &KeyModifiers::NONE) => {
                    if self.input.len() > 0 && self.cursor > 0 {
                        self.input.remove(self.cursor - 1);
                        self.cursor -= 1;
                    }
                }
                (Left, &KeyModifiers::NONE)
                | (Char('h'), &KeyModifiers::CONTROL | &KeyModifiers::ALT) => {
                    self.cursor = max(self.cursor, 1) - 1;
                    // let actual_cursor = self.input.chars()
                    //
                    // let prev_boundry = self.input[..self.cursor]
                    //     .char_indices()
                    //     .rfind(|item| self.input.is_char_boundary(item.0));
                    // if let Some(p) = prev_boundry {
                    //     self.cursor = (p.0 + 1) - p.1.width().unwrap_or(0);
                    // } else {
                    //     self.cursor = 0;
                    // }
                }
                (Right, &KeyModifiers::NONE)
                | (Char('l'), &KeyModifiers::CONTROL | &KeyModifiers::ALT) => {
                    self.cursor = min(self.cursor + 1, self.input.len());
                }
                (End, &KeyModifiers::NONE) | (Char('e'), &KeyModifiers::CONTROL) => {
                    self.cursor = self.input.len();
                }
                (Home, &KeyModifiers::NONE) | (Char('a'), &KeyModifiers::CONTROL) => {
                    self.cursor = 0;
                }
                (Enter, &KeyModifiers::NONE) => {
                    app.mode = Mode::Loading;
                }
                _ => {}
            };
        }
    }
}

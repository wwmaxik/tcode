pub mod pty;

use ratatui::style::{Color, Modifier, Style};
use vte::{Params, Perform};

#[derive(Clone, Debug)]
pub struct TermCell {
    pub char: char,
    pub style: Style,
}

pub struct TerminalState {
    pub lines: Vec<Vec<TermCell>>,
    pub cursor_x: usize,
    pub cursor_y: usize,    // Absolute row in lines
    pub screen_base: usize, // First visible row in lines
    pub width: usize,
    pub height: usize,
    pub current_style: Style,
    pub scroll_offset: usize, // User scroll offset from bottom
}

impl TerminalState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            lines: vec![Vec::new()],
            cursor_x: 0,
            cursor_y: 0,
            screen_base: 0,
            width,
            height,
            current_style: Style::default(),
            scroll_offset: 0,
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
    }
}

impl Perform for TerminalState {
    fn print(&mut self, c: char) {
        let y = self.cursor_y;
        if y >= self.lines.len() {
            self.lines.resize(y + 1, Vec::new());
        }
        let line = &mut self.lines[y];
        if self.cursor_x >= line.len() {
            line.resize(
                self.cursor_x + 1,
                TermCell {
                    char: ' ',
                    style: self.current_style,
                },
            );
        }
        line[self.cursor_x] = TermCell {
            char: c,
            style: self.current_style,
        };
        self.cursor_x += 1;
        if self.cursor_x >= self.width {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.screen_base + self.height {
                self.screen_base = self.cursor_y - self.height + 1;
            }
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.cursor_y += 1;
                if self.cursor_y >= self.screen_base + self.height {
                    self.screen_base = self.cursor_y - self.height + 1;
                }
            }
            b'\r' => self.cursor_x = 0,
            b'\t' => {
                let next_tab = (self.cursor_x / 8 + 1) * 8;
                self.cursor_x = next_tab.min(self.width.saturating_sub(1));
            }
            b'\x08' => self.cursor_x = self.cursor_x.saturating_sub(1), // Backspace
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        match action {
            'm' => {
                // SGR (Select Graphic Rendition)
                if params.is_empty() {
                    self.current_style = Style::default();
                    return;
                }
                for param in params {
                    let mut iter = param.iter();
                    for &code in iter {
                        match code {
                            0 => self.current_style = Style::default(),
                            1 => {
                                self.current_style = self.current_style.add_modifier(Modifier::BOLD)
                            }
                            30..=37 => {
                                let colors = [
                                    Color::Black,
                                    Color::Red,
                                    Color::Green,
                                    Color::Yellow,
                                    Color::Blue,
                                    Color::Magenta,
                                    Color::Cyan,
                                    Color::Gray,
                                ];
                                self.current_style =
                                    self.current_style.fg(colors[(code - 30) as usize]);
                            }
                            39 => self.current_style.fg = None,
                            40..=47 => {
                                let colors = [
                                    Color::Black,
                                    Color::Red,
                                    Color::Green,
                                    Color::Yellow,
                                    Color::Blue,
                                    Color::Magenta,
                                    Color::Cyan,
                                    Color::Gray,
                                ];
                                self.current_style =
                                    self.current_style.bg(colors[(code - 40) as usize]);
                            }
                            49 => self.current_style.bg = None,
                            90..=97 => {
                                // Bright fg
                                let colors = [
                                    Color::DarkGray,
                                    Color::LightRed,
                                    Color::LightGreen,
                                    Color::LightYellow,
                                    Color::LightBlue,
                                    Color::LightMagenta,
                                    Color::LightCyan,
                                    Color::White,
                                ];
                                self.current_style =
                                    self.current_style.fg(colors[(code - 90) as usize]);
                            }
                            100..=107 => {
                                // Bright bg
                                let colors = [
                                    Color::DarkGray,
                                    Color::LightRed,
                                    Color::LightGreen,
                                    Color::LightYellow,
                                    Color::LightBlue,
                                    Color::LightMagenta,
                                    Color::LightCyan,
                                    Color::White,
                                ];
                                self.current_style =
                                    self.current_style.bg(colors[(code - 100) as usize]);
                            }
                            _ => {}
                        }
                    }
                }
            }
            'A' => {
                // Cursor Up
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                self.cursor_y = self.cursor_y.saturating_sub(n).max(self.screen_base);
            }
            'B' => {
                // Cursor Down
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                self.cursor_y = (self.cursor_y + n).min(self.screen_base + self.height - 1);
            }
            'C' => {
                // Cursor Forward
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                self.cursor_x += n;
            }
            'D' => {
                // Cursor Back
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                self.cursor_x = self.cursor_x.saturating_sub(n);
            }
            'K' => {
                // Erase in Line
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(0);
                if self.cursor_y >= self.lines.len() {
                    self.lines.resize(self.cursor_y + 1, Vec::new());
                }
                if mode == 0 {
                    self.lines[self.cursor_y].truncate(self.cursor_x);
                } else if mode == 2 {
                    self.lines[self.cursor_y].clear();
                }
            }
            'J' => {
                // Erase in Display
                let mode = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(0);
                if mode == 2 {
                    // Clear entire screen
                    for i in 0..self.height {
                        let y = self.screen_base + i;
                        if y < self.lines.len() {
                            self.lines[y].clear();
                        }
                    }
                    self.cursor_y = self.screen_base;
                    self.cursor_x = 0;
                }
            }
            'H' | 'f' => {
                // Cursor Position
                let mut it = params.iter();
                let row = it
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                let col = it
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                self.cursor_y = self.screen_base + row.saturating_sub(1);
                self.cursor_x = col.saturating_sub(1);
            }
            'L' => {
                // Insert Line
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                for _ in 0..n {
                    if self.cursor_y < self.lines.len() {
                        self.lines.insert(self.cursor_y, Vec::new());
                    }
                }
            }
            'M' => {
                // Delete Line
                let n = params
                    .iter()
                    .next()
                    .and_then(|p| p.iter().next().copied())
                    .unwrap_or(1) as usize;
                for _ in 0..n {
                    if self.cursor_y < self.lines.len() {
                        self.lines.remove(self.cursor_y);
                    }
                }
            }
            _ => {}
        }
    }
}

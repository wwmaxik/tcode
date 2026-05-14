use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

use crate::ai;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use crate::app::{App, InputMode, SidebarPanel};
use crate::events::UiRects;

#[allow(dead_code)]
mod theme {
    use ratatui::style::Color;
    pub const BG: Color = Color::Rgb(30, 30, 46);
    pub const SURFACE: Color = Color::Rgb(36, 39, 58);
    pub const BORDER: Color = Color::Rgb(69, 71, 90);
    pub const BORDER_ACTIVE: Color = Color::Rgb(137, 180, 250);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const TEXT_DIM: Color = Color::Rgb(127, 132, 156);
    pub const ACCENT: Color = Color::Rgb(137, 180, 250);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const LINE_NUM: Color = Color::Rgb(88, 91, 112);
    pub const LINE_NUM_ACTIVE: Color = Color::Rgb(205, 214, 244);
    pub const CURSOR_LINE_BG: Color = Color::Rgb(45, 47, 65);
    pub const TAB_ACTIVE_BG: Color = Color::Rgb(30, 30, 46);
    pub const TAB_INACTIVE_BG: Color = Color::Rgb(24, 24, 37);
    pub const STATUS_BG: Color = Color::Rgb(24, 24, 37);
    pub const EXPLORER_BG: Color = Color::Rgb(24, 24, 37);
    pub const EXPLORER_SEL: Color = Color::Rgb(45, 47, 65);
    pub const DIR_COLOR: Color = Color::Rgb(137, 180, 250);
    pub const FILE_COLOR: Color = Color::Rgb(205, 214, 244);
    pub const OVERLAY_BG: Color = Color::Rgb(36, 39, 58);
    pub const OVERLAY_BORDER: Color = Color::Rgb(137, 180, 250);
    pub const AI_BG: Color = Color::Rgb(24, 24, 37);
    pub const AI_USER_BG: Color = Color::Rgb(45, 50, 80);
    pub const AI_TOOL_BG: Color = Color::Rgb(35, 40, 55);
    pub const AI_CODE_BG: Color = Color::Rgb(40, 42, 58);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
}

/// Cached span data for a single highlighted text segment.
struct HlSpan {
    fg: Color,
    modifier: Modifier,
    text: String,
}

/// Syntax highlighter with built-in result cache.
/// Re-highlights only when the active tab or its text_version changes.
pub struct Highlighter {
    ss: SyntaxSet,
    ts: ThemeSet,
    cache: Vec<Vec<HlSpan>>,
    cache_tab: usize,
    cache_version: u64,
    cache_theme: String,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            ss: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
            cache: Vec::new(),
            cache_tab: usize::MAX,
            cache_version: u64::MAX,
            cache_theme: String::new(),
        }
    }

    /// Return sorted list of available theme names.
    pub fn theme_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.ts.themes.keys().cloned().collect();
        names.sort();
        names
    }

    fn ensure_cache(&mut self, app: &App) {
        let tab = &app.tabs[app.active_tab];
        if self.cache_tab == app.active_tab
            && self.cache_version == tab.text_version
            && self.cache_theme == app.current_theme
        {
            return;
        }

        let ext = tab
            .file_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("rs");

        let syntax = self
            .ss
            .find_syntax_by_extension(ext)
            .unwrap_or_else(|| self.ss.find_syntax_plain_text());
        let tm = self
            .ts
            .themes
            .get(&app.current_theme)
            .unwrap_or_else(|| &self.ts.themes["base16-ocean.dark"]);

        let mut h = HighlightLines::new(syntax, tm);
        let full_text = tab.lines.join("\n") + "\n";

        self.cache.clear();
        for line_str in LinesWithEndings::from(&full_text) {
            let ranges = h.highlight_line(line_str, &self.ss).unwrap_or_default();
            let spans: Vec<HlSpan> = ranges
                .iter()
                .map(|(style, text)| {
                    let fg = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
                    let mut m = Modifier::empty();
                    if style.font_style.contains(FontStyle::BOLD) {
                        m |= Modifier::BOLD;
                    }
                    if style.font_style.contains(FontStyle::ITALIC) {
                        m |= Modifier::ITALIC;
                    }
                    HlSpan {
                        fg,
                        modifier: m,
                        text: text.trim_end_matches('\n').to_string(),
                    }
                })
                .collect();
            self.cache.push(spans);
        }

        self.cache_tab = app.active_tab;
        self.cache_version = tab.text_version;
        self.cache_theme = app.current_theme.clone();
    }
}

// ─── Public draw entry point ────────────────────────────────────────

pub fn draw(f: &mut Frame, app: &App, hl: &mut Highlighter) -> UiRects {
    let size = f.area();
    f.render_widget(Block::default().style(Style::default().bg(theme::BG)), size);

    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tab bar
            Constraint::Min(1),    // main
            Constraint::Length(1), // status
        ])
        .split(size);

    draw_tab_bar(f, app, outer[0]);

    let explorer_w: u16 = 24;
    let ai_panel_w: u16 = if app.show_ai_panel { 50 } else { 0 };
    let main_area = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(7), // Activity Bar
            Constraint::Length(if app.config.show_explorer {
                explorer_w
            } else {
                0
            }),
            Constraint::Min(1),             // Editor
            Constraint::Length(ai_panel_w), // AI Panel (right)
        ])
        .split(outer[1]);

    draw_activity_bar(f, app, main_area[0]);

    let _explorer_rect = match app.sidebar_panel {
        SidebarPanel::Explorer => {
            if app.config.show_explorer {
                draw_explorer(f, app, main_area[1])
            } else {
                (0, 0, 0, 0)
            }
        }
        SidebarPanel::Git => {
            if app.config.show_explorer {
                draw_git_panel(f, app, main_area[1])
            } else {
                (0, 0, 0, 0)
            }
        }
        _ => {
            if app.config.show_explorer {
                f.render_widget(
                    Block::default()
                        .borders(Borders::RIGHT)
                        .border_style(Style::default().fg(theme::BORDER)),
                    main_area[1],
                );
            }
            (0, 0, 0, 0)
        }
    };

    // Ensure highlight cache is fresh before drawing editor
    hl.ensure_cache(app);

    let mut editor_area = main_area[2];
    let mut terminal_area = Rect::default();

    if app.show_terminal {
        let v_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(main_area[2]);
        editor_area = v_split[0];
        terminal_area = v_split[1];
    }

    let text_area_rect = draw_editor(f, app, editor_area, &hl.cache);

    if app.show_terminal {
        draw_terminal(f, app, terminal_area);
    }

    // AI Panel (right sidebar)
    let mut ai_panel_rect = crate::events::AiPanelRect::default();
    if app.show_ai_panel {
        ai_panel_rect = draw_ai_panel(f, app, main_area[3]);
    }

    draw_status_bar(f, app, outer[2]);

    // Modal overlays
    let mut theme_modal_rect = crate::events::ModalRect::default();
    if app.show_help {
        draw_help_overlay(f, size);
    }
    if app.show_theme_picker {
        theme_modal_rect = draw_theme_picker(f, app, size);
    }
    if app.show_plugin_manager {
        draw_plugin_manager(f, app, size);
    }
    if app.show_fuzzy_finder {
        draw_fuzzy_finder(f, app, size);
    }
    if app.input_mode == InputMode::FolderPrompt {
        draw_folder_prompt(f, app, outer[2]);
    }
    if app.show_settings {
        draw_settings_modal(f, app, size);
    }
    if app.input_mode == InputMode::Prompt {
        draw_generic_prompt(f, app, size);
    }

    UiRects {
        text_area: crate::events::TextAreaRect {
            x: text_area_rect.0,
            y: text_area_rect.1,
            width: text_area_rect.2,
            height: text_area_rect.3,
        },
        explorer: crate::events::ExplorerRect {
            x: _explorer_rect.0,
            y: _explorer_rect.1,
            width: _explorer_rect.2,
            height: _explorer_rect.3,
        },
        tab_bar: crate::events::TabBarRect {
            x: outer[0].x,
            y: outer[0].y,
            width: outer[0].width,
            height: outer[0].height,
        },
        theme_modal: theme_modal_rect,
        terminal: crate::events::TerminalRect {
            width: terminal_area.width,
            height: terminal_area.height,
        },
        activity_bar: crate::events::ActivityBarRect {
            x: main_area[0].x,
            y: main_area[0].y,
            width: main_area[0].width,
            height: main_area[0].height,
        },
        ai_panel: ai_panel_rect,
    }
}

// ─── Tab bar ────────────────────────────────────────────────────────

fn draw_tab_bar(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = app
        .tabs
        .iter()
        .enumerate()
        .map(|(i, tab)| {
            let marker = if tab.dirty { " ●" } else { "" };
            let label = format!(" {}{} ", tab.name, marker);
            if i == app.active_tab {
                Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(theme::TEXT)
                        .bg(theme::TAB_ACTIVE_BG)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    label,
                    Style::default()
                        .fg(theme::TEXT_DIM)
                        .bg(theme::TAB_INACTIVE_BG),
                ))
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .select(app.active_tab)
        .style(Style::default().bg(theme::TAB_INACTIVE_BG))
        .highlight_style(
            Style::default()
                .fg(theme::TEXT)
                .bg(theme::TAB_ACTIVE_BG)
                .add_modifier(Modifier::BOLD),
        )
        .divider("│");
    f.render_widget(tabs, area);
}

// ─── Activity Bar ───────────────────────────────────────────────────

fn draw_activity_bar(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(theme::BORDER))
        .style(Style::default().bg(theme::STATUS_BG)); // Slightly different background
    f.render_widget(block, area);

    let items = vec![
        (SidebarPanel::Explorer, "󰉋"),
        (SidebarPanel::Git, "󰊢"),
        (SidebarPanel::Search, "󰍉"),
        (SidebarPanel::Plugins, "󰈺"),
    ];

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Explorer
            Constraint::Length(3), // Git
            Constraint::Length(3), // Search
            Constraint::Length(3), // Plugins
            Constraint::Min(0),    // Space
            Constraint::Length(3), // Settings
            Constraint::Length(3), // Help
        ])
        .split(area);

    for (i, (panel, icon)) in items.into_iter().enumerate() {
        let style = if app.sidebar_panel == panel {
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT_DIM)
        };
        // Vertical centering within the 3-line cell
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .split(layout[i]);

        f.render_widget(
            Paragraph::new(format!("  {}", icon.trim())).style(style),
            rows[1],
        );
    }

    // Settings icon at bottom
    let settings_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(layout[5]);
    f.render_widget(
        Paragraph::new("  󰒓").style(Style::default().fg(theme::TEXT_DIM)),
        settings_rows[1],
    );

    // Help icon at bottom
    let help_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(layout[6]);
    f.render_widget(
        Paragraph::new("  󰞋").style(Style::default().fg(theme::TEXT_DIM)),
        help_rows[1],
    );
}

// ─── File explorer ──────────────────────────────────────────────────

fn draw_explorer(f: &mut Frame, app: &App, area: Rect) -> (u16, u16, u16, u16) {
    let bc = if app.focus == crate::app::Focus::Explorer {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER
    };
    let cwd_name = app
        .cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| app.cwd.to_string_lossy().to_string());

    let block = Block::default()
        .title(format!(" {} ", cwd_name))
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(bc))
        .style(Style::default().bg(theme::EXPLORER_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let vh = inner.height as usize;
    let scroll = if app.file_tree_selected >= app.file_tree_scroll + vh {
        app.file_tree_selected - vh + 1
    } else if app.file_tree_selected < app.file_tree_scroll {
        app.file_tree_selected
    } else {
        app.file_tree_scroll
    };

    let items: Vec<ListItem> = app
        .file_tree
        .iter()
        .enumerate()
        .skip(scroll)
        .take(vh)
        .map(|(i, entry)| {
            let icon = if entry.is_dir {
                "󰉋 "
            } else {
                file_icon(&entry.name)
            };
            let color = if entry.is_dir {
                theme::DIR_COLOR
            } else {
                theme::FILE_COLOR
            };
            let style = if i == app.file_tree_selected {
                Style::default().fg(color).bg(theme::EXPLORER_SEL)
            } else {
                Style::default().fg(color)
            };
            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::styled(&entry.name, style),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);
    (inner.x, inner.y, inner.width, inner.height)
}

// ─── Git Panel ──────────────────────────────────────────────────────

fn draw_git_panel(f: &mut Frame, app: &App, area: Rect) -> (u16, u16, u16, u16) {
    let bc = if app.focus == crate::app::Focus::Explorer {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER
    };
    let block = Block::default()
        .title(" SOURCE CONTROL ")
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(bc))
        .style(Style::default().bg(theme::EXPLORER_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.git_repo.is_none() {
        f.render_widget(
            Paragraph::new("No git repo found").style(Style::default().fg(theme::TEXT_DIM)),
            inner,
        );
        return (inner.x, inner.y, inner.width, inner.height);
    }

    if app.git_changes.is_empty() {
        f.render_widget(
            Paragraph::new(" No changes detected").style(Style::default().fg(theme::TEXT_DIM)),
            inner,
        );
        return (inner.x, inner.y, inner.width, inner.height);
    }

    let items: Vec<ListItem> = app
        .git_changes
        .iter()
        .enumerate()
        .map(|(i, change)| {
            let style = if i == app.git_selected {
                Style::default()
                    .fg(theme::BG)
                    .bg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };

            let status_style = match change.status.as_str() {
                "M" => Style::default().fg(theme::YELLOW),
                "A" => Style::default().fg(theme::GREEN),
                "D" => Style::default().fg(theme::RED),
                _ => Style::default().fg(theme::TEXT_DIM),
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!(" {} ", change.status), status_style),
                Span::styled(&change.path, style),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);

    (inner.x, inner.y, inner.width, inner.height)
}

fn file_icon(name: &str) -> &'static str {
    match name.rsplit('.').next() {
        Some("rs") => "󱘗 ",
        Some("toml") => " ",
        Some("md") => "󰍔 ",
        Some("json") => " ",
        Some("js" | "jsx") => "󰌞 ",
        Some("ts" | "tsx") => "󰛦 ",
        Some("py") => "󰌠 ",
        Some("go") => "󰟓 ",
        Some("html") => "󰌝 ",
        Some("css") => "󰌜 ",
        Some("lock") => "󰌾 ",
        _ => "󰈙 ",
    }
}

// ─── Editor ─────────────────────────────────────────────────────────

fn is_selected(r: usize, c: usize, start: (usize, usize), end: (usize, usize)) -> bool {
    let (s_r, s_c, e_r, e_c) = if start < end {
        (start.0, start.1, end.0, end.1)
    } else {
        (end.0, end.1, start.0, start.1)
    };
    if r < s_r || r > e_r {
        return false;
    }
    if s_r == e_r {
        return c >= s_c && c < e_c;
    }
    if r == s_r {
        return c >= s_c;
    }
    if r == e_r {
        return c < e_c;
    }
    true
}

fn draw_editor(
    f: &mut Frame,
    app: &App,
    area: Rect,
    hl_cache: &[Vec<HlSpan>],
) -> (u16, u16, u16, u16) {
    let block = Block::default().style(Style::default().bg(theme::BG));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let tab = &app.tabs[app.active_tab];
    let total = tab.lines.len();
    let gw = format!("{}", total).len() as u16 + 4;

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(gw), Constraint::Min(1)])
        .split(inner);

    let gutter_area = cols[0];
    let text_area = cols[1];
    let vh = text_area.height as usize;
    let vw = text_area.width as usize;

    // ── Gutter ──
    let gutter_lines: Vec<Line> = (0..vh)
        .map(|vi| {
            let li = tab.scroll_offset + vi;
            if li < total {
                let has_error = app
                    .lsp_diagnostics
                    .iter()
                    .any(|d| li >= d.range.start.line as usize && li <= d.range.end.line as usize);
                let mark = if has_error {
                    Some('E')
                } else if li < tab.git_marks.len() {
                    tab.git_marks[li]
                } else {
                    None
                };
                let (m_char, m_color) = match mark {
                    Some('E') => ("E", theme::RED),
                    Some('+') => ("+", theme::GREEN),
                    Some('~') => ("~", theme::YELLOW),
                    Some('|') => ("|", theme::RED),
                    _ => (" ", theme::BG),
                };
                let num = format!("{:>w$} ", li + 1, w = (gw - 3) as usize);
                let color = if li == tab.cursor_row {
                    theme::LINE_NUM_ACTIVE
                } else {
                    theme::LINE_NUM
                };
                let bg = if li == tab.cursor_row {
                    theme::CURSOR_LINE_BG
                } else {
                    theme::BG
                };
                Line::from(vec![
                    Span::styled(m_char, Style::default().fg(m_color).bg(bg)),
                    Span::styled(num, Style::default().fg(color).bg(bg)),
                ])
            } else {
                Line::from(Span::styled(
                    format!("{:>w$} ", "~", w = (gw - 1) as usize),
                    Style::default().fg(theme::LINE_NUM).bg(theme::BG),
                ))
            }
        })
        .collect();
    f.render_widget(Paragraph::new(gutter_lines), gutter_area);

    // ── Text with cached syntax highlighting ──
    let text_lines: Vec<Line> = (0..vh)
        .map(|vi| {
            let li = tab.scroll_offset + vi;
            let bg = if li == tab.cursor_row {
                theme::CURSOR_LINE_BG
            } else {
                theme::BG
            };

            if li < total && li < hl_cache.len() {
                let mut spans: Vec<Span> = Vec::new();
                let mut col_offset: usize = 0;

                for hl_span in &hl_cache[li] {
                    let text_len = hl_span.text.len();
                    if text_len == 0 {
                        continue;
                    }

                    if col_offset + text_len <= tab.scroll_x {
                        col_offset += text_len;
                        continue;
                    }

                    let start_in_span = tab.scroll_x.saturating_sub(col_offset);
                    let visible_text = &hl_span.text[start_in_span..];

                    for (char_idx, ch) in visible_text.chars().enumerate() {
                        let absolute_col = col_offset + start_in_span + char_idx;
                        let mut final_style = Style::default()
                            .fg(hl_span.fg)
                            .bg(bg)
                            .add_modifier(hl_span.modifier);

                        if let Some(sel_start) = tab.selection_start
                            && is_selected(
                                li,
                                absolute_col,
                                sel_start,
                                (tab.cursor_row, tab.cursor_col),
                            ) {
                                final_style = final_style.bg(theme::ACCENT).fg(theme::BG);
                            }
                        spans.push(Span::styled(ch.to_string(), final_style));
                    }
                    col_offset += text_len;
                }

                // Pad to fill width
                let visible_len: usize = spans.iter().map(|s| s.content.len()).sum();
                if visible_len < vw {
                    spans.push(Span::styled(
                        " ".repeat(vw - visible_len),
                        Style::default().bg(bg),
                    ));
                }

                Line::from(spans)
            } else {
                Line::from(Span::styled(" ".repeat(vw), Style::default().bg(bg)))
            }
        })
        .collect();

    f.render_widget(Paragraph::new(text_lines), text_area);

    // ── Cursor ──
    if app.focus == crate::app::Focus::Editor
        && app.input_mode == InputMode::Editing
        && !app.show_help
    {
        let cr = tab.cursor_row as i64 - tab.scroll_offset as i64;
        let cc = tab.cursor_col as i64 - tab.scroll_x as i64;
        if cr >= 0 && (cr as u16) < text_area.height && cc >= 0 && (cc as u16) < text_area.width {
            f.set_cursor_position((text_area.x + cc as u16, text_area.y + cr as u16));
        }
    }

    // ── Autocomplete Modal ──
    if app.focus == crate::app::Focus::Editor && app.input_mode == InputMode::Editing
        && let Some(completions) = &app.lsp_completions
            && !completions.is_empty() {
                let max_w = 40u16;
                let max_h = 10u16;
                let items_len = completions.len() as u16;
                let h = items_len.min(max_h) + 2;

                let cr = tab.cursor_row as i64 - tab.scroll_offset as i64;
                let cc = tab.cursor_col as i64 - tab.scroll_x as i64;

                if cr >= 0
                    && (cr as u16) < text_area.height
                    && cc >= 0
                    && (cc as u16) < text_area.width
                {
                    let mut cx = text_area.x + cc as u16;
                    let mut cy = text_area.y + cr as u16 + 1;

                    if cx + max_w > text_area.right() {
                        cx = text_area.right().saturating_sub(max_w);
                    }
                    if cy + h > text_area.bottom() {
                        cy = text_area.y + (cr as u16).saturating_sub(h);
                    }

                    let rect = Rect::new(cx, cy, max_w, h);
                    f.render_widget(Clear, rect);
                    let block = Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme::BORDER_ACTIVE))
                        .style(Style::default().bg(theme::OVERLAY_BG));

                    let scroll = app
                        .lsp_completion_selected
                        .saturating_sub((max_h / 2) as usize);
                    let items: Vec<ListItem> = completions
                        .iter()
                        .enumerate()
                        .skip(scroll)
                        .take(max_h as usize)
                        .map(|(i, c)| {
                            let style = if i == app.lsp_completion_selected {
                                Style::default()
                                    .fg(theme::BG)
                                    .bg(theme::ACCENT)
                                    .add_modifier(Modifier::BOLD)
                            } else {
                                Style::default().fg(theme::TEXT)
                            };
                            let label = c.label.as_str();
                            let truncated = if label.len() > 36 {
                                &label[..36]
                            } else {
                                label
                            };
                            ListItem::new(Line::from(Span::styled(
                                format!(" {}", truncated),
                                style,
                            )))
                        })
                        .collect();

                    f.render_widget(List::new(items).block(block), rect);
                }
            }

    (text_area.x, text_area.y, text_area.width, text_area.height)
}

// ─── Status bar ─────────────────────────────────────────────────────

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let tab = &app.tabs[app.active_tab];

    let git_branch = if let Some(repo) = &app.git_repo {
        if let Ok(head) = repo.head() {
            if let Some(name) = head.shorthand() {
                format!("  {} │", name)
            } else {
                "  detached │".to_string()
            }
        } else {
            "  init │".to_string()
        }
    } else {
        String::new()
    };

    let mode = Span::styled(
        " INSERT ",
        Style::default()
            .fg(theme::BG)
            .bg(theme::GREEN)
            .add_modifier(Modifier::BOLD),
    );
    let file = Span::styled(
        format!(" {} ", tab.name),
        Style::default().fg(theme::TEXT).bg(theme::STATUS_BG),
    );
    let git = Span::styled(
        git_branch.clone(),
        Style::default().fg(theme::GREEN).bg(theme::STATUS_BG),
    );
    let pos = format!(" Ln {}, Col {} ", tab.cursor_row + 1, tab.cursor_col + 1);
    let pos_len = pos.len();
    let pos_span = Span::styled(
        pos,
        Style::default().fg(theme::TEXT_DIM).bg(theme::STATUS_BG),
    );
    let status = Span::styled(
        format!(" {} ", app.status_msg),
        Style::default().fg(theme::YELLOW).bg(theme::STATUS_BG),
    );

    let shortcuts = " ^H Help │ ^O Folder │ ^P Fuzzy │ ^N Tabs ";
    let sc = Span::styled(
        shortcuts,
        Style::default().fg(theme::TEXT_DIM).bg(theme::STATUS_BG),
    );

    let left = 8 + tab.name.len() + 2 + git_branch.len() + pos_len + app.status_msg.len() + 2;
    let gap = (area.width as usize).saturating_sub(left + shortcuts.len());
    let gap_span = Span::styled(" ".repeat(gap), Style::default().bg(theme::STATUS_BG));

    let line = Line::from(vec![mode, file, git, pos_span, status, gap_span, sc]);
    f.render_widget(
        Paragraph::new(line).style(Style::default().bg(theme::STATUS_BG)),
        area,
    );
}

// ─── Help overlay ───────────────────────────────────────────────────

fn draw_help_overlay(f: &mut Frame, size: Rect) {
    let w = 52.min(size.width.saturating_sub(4));
    let h = 26.min(size.height.saturating_sub(4));
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" ⌨ Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY_BORDER))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let s = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled(
                format!("  {:10}", key),
                Style::default()
                    .fg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(desc.to_string(), Style::default().fg(theme::TEXT)),
        ])
    };

    let help_text = vec![
        s("Ctrl+Q", "Quit"),
        s("Ctrl+S", "Save file"),
        s("Ctrl+O", "Open folder"),
        s("Ctrl+B", "Toggle explorer"),
        s("Ctrl+W", "Close tab"),
        s("Ctrl+N", "New/Next tab"),
        s("Ctrl+H", "This help"),
        s("Ctrl+T", "Theme switcher"),
        s("Ctrl+U", "Plugin manager"),
        s("Ctrl+L", "Open settings"),
        s("Ctrl+~ / J", "Toggle terminal"),
        s("Ctrl+P", "Fuzzy Finder (Files)"),
        Line::from(""),
        Line::from(Span::styled(
            "  ── Tabs ──",
            Style::default().fg(theme::YELLOW),
        )),
        s("Ctrl+N", "Next tab"),
        s("Ctrl+W", "Close tab"),
        s("Alt+←/→", "Switch tabs"),
        Line::from(""),
        Line::from(Span::styled(
            "  ── Editor ──",
            Style::default().fg(theme::YELLOW),
        )),
        s("Arrows", "Move cursor"),
        s("Home/End", "Start / end of line"),
        s("Esc", "Focus explorer"),
        Line::from(""),
        Line::from(Span::styled(
            "  ── Terminal ──",
            Style::default().fg(theme::YELLOW),
        )),
        s("Ctrl+~ / J", "Toggle"),
        s("Shift+Up/Dn", "Scrollback"),
        Line::from(""),
        Line::from(Span::styled(
            "  ── LSP / IDE ──",
            Style::default().fg(theme::YELLOW),
        )),
        s("Up/Dn/Ent", "Autocomplete (when active)"),
        s("Gutter 'E'", "Diagnostic markers"),
        Line::from(""),
        Line::from(Span::styled(
            "  ── Mouse ──",
            Style::default().fg(theme::YELLOW),
        )),
        Line::from(Span::styled(
            "  Click editor / explorer / tabs",
            Style::default().fg(theme::TEXT),
        )),
        Line::from(Span::styled(
            "  Middle-click tab to close it",
            Style::default().fg(theme::TEXT),
        )),
        Line::from(Span::styled(
            "  Scroll wheel to scroll buffer",
            Style::default().fg(theme::TEXT),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_text)
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );
}

// ─── Folder prompt ──────────────────────────────────────────────────

fn draw_folder_prompt(f: &mut Frame, app: &App, status_area: Rect) {
    let prompt_area = Rect::new(0, status_area.y, f.area().width, 1);
    f.render_widget(Clear, prompt_area);

    let label = Span::styled(
        " Open Folder: ",
        Style::default()
            .fg(theme::BG)
            .bg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    );
    let input = Span::styled(
        &app.input_buffer,
        Style::default().fg(theme::TEXT).bg(theme::STATUS_BG),
    );
    let pad_len = (prompt_area.width as usize).saturating_sub(14 + app.input_buffer.len());
    let pad = Span::styled(" ".repeat(pad_len), Style::default().bg(theme::STATUS_BG));

    f.render_widget(
        Paragraph::new(Line::from(vec![label, input, pad])),
        prompt_area,
    );

    let cx = 14 + app.input_cursor as u16;
    if cx < prompt_area.width {
        f.set_cursor_position((prompt_area.x + cx, prompt_area.y));
    }
}

// ─── Theme picker modal ─────────────────────────────────────────────

fn draw_theme_picker(f: &mut Frame, app: &App, size: Rect) -> crate::events::ModalRect {
    let count = app.theme_list.len();
    let w = 40.min(size.width.saturating_sub(4));
    let h = (count as u16 + 2).min(size.height.saturating_sub(4));
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 🎨 Theme Switcher ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY_BORDER))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = app
        .theme_list
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let marker = if *name == app.current_theme {
                " ● "
            } else {
                "   "
            };
            let style = if i == app.theme_selected {
                Style::default()
                    .fg(theme::BG)
                    .bg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else if *name == app.current_theme {
                Style::default().fg(theme::GREEN)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}", marker, name),
                style,
            )))
        })
        .collect();

    f.render_widget(List::new(items), inner);

    crate::events::ModalRect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height,
        active: true,
    }
}

// ─── Plugin manager modal ───────────────────────────────────────────

fn draw_plugin_manager(f: &mut Frame, app: &App, size: Rect) {
    let count = app.plugin_info.len();
    let w = 48.min(size.width.saturating_sub(4));
    let h = (count as u16 + 4).min(size.height.saturating_sub(4)).max(6);
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 🔌 Plugin Manager ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY_BORDER))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.plugin_info.is_empty() {
        let msg = Line::from(Span::styled(
            "  No plugins found in plugins/",
            Style::default().fg(theme::TEXT_DIM),
        ));
        f.render_widget(Paragraph::new(vec![Line::from(""), msg]), inner);
        return;
    }

    let items: Vec<ListItem> = app
        .plugin_info
        .iter()
        .map(|(name, loaded)| {
            let status = if *loaded { "✓ Loaded" } else { "✗ Error" };
            let status_color = if *loaded { theme::GREEN } else { theme::RED };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<28}", name), Style::default().fg(theme::TEXT)),
                Span::styled(status.to_string(), Style::default().fg(status_color)),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), inner);
}

// ─── Terminal ───────────────────────────────────────────────────────

fn draw_terminal(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Terminal (Ctrl+~) ")
        .borders(Borders::ALL)
        .border_style(
            Style::default().fg(if app.focus == crate::app::Focus::Terminal {
                theme::BORDER_ACTIVE
            } else {
                theme::BORDER
            }),
        )
        .style(Style::default().bg(theme::BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let ts = &app.terminal;
    let mut lines = Vec::new();

    // Compute visible range
    let max_lines = inner.height as usize;
    let start_idx = if ts.scroll_offset == 0 {
        ts.screen_base
    } else {
        ts.lines.len().saturating_sub(max_lines + ts.scroll_offset)
    };
    let end_idx = (start_idx + max_lines).min(ts.lines.len());

    for i in start_idx..end_idx {
        if i < ts.lines.len() {
            let row = &ts.lines[i];
            let mut spans = Vec::new();
            for cell in row {
                spans.push(Span::styled(cell.char.to_string(), cell.style));
            }
            lines.push(Line::from(spans));
        }
    }

    f.render_widget(Paragraph::new(lines), inner);

    if app.focus == crate::app::Focus::Terminal {
        let cursor_y = ts.cursor_y.saturating_sub(start_idx) as u16;
        let cursor_x = ts.cursor_x as u16;
        if cursor_y < inner.height && cursor_x < inner.width {
            f.set_cursor_position((inner.x + cursor_x, inner.y + cursor_y));
        }
    }
}

// ─── Fuzzy Finder ───────────────────────────────────────────────────

fn draw_fuzzy_finder(f: &mut Frame, app: &App, size: Rect) {
    let w = 60.min(size.width.saturating_sub(4));
    let h = 20.min(size.height.saturating_sub(4));
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" 🔍 Fuzzy Finder ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY_BORDER))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1)])
        .split(inner);

    let input_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(Style::default().fg(theme::BORDER));
    let input = Paragraph::new(format!("> {}_", app.fuzzy_query))
        .style(Style::default().fg(theme::TEXT).bg(theme::OVERLAY_BG))
        .block(input_block);
    f.render_widget(input, chunks[0]);

    let items: Vec<ListItem> = app
        .fuzzy_results
        .iter()
        .enumerate()
        .map(|(i, res)| {
            let style = if i == app.fuzzy_selected {
                Style::default()
                    .fg(theme::BG)
                    .bg(theme::GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT).bg(theme::OVERLAY_BG)
            };
            ListItem::new(Line::from(Span::styled(format!("  {}", res), style)))
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(list, chunks[1]);
}

fn draw_settings_modal(f: &mut Frame, app: &App, size: Rect) {
    let count = 4;
    let w = 50.min(size.width.saturating_sub(4));
    let h = (count as u16 + 2).min(size.height.saturating_sub(4));
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .title(" ⚙ Settings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::OVERLAY_BORDER))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let items = vec![
        ("Tab Size", app.config.tab_size.to_string()),
        (
            "Autosave Interval",
            format!("{}ms", app.config.autosave_interval_ms),
        ),
        ("Show Explorer", app.config.show_explorer.to_string()),
        ("Current Theme", app.config.theme.clone()),
    ];

    let list_items: Vec<ListItem> = items
        .into_iter()
        .enumerate()
        .map(|(i, (name, val))| {
            let style = if i == app.settings_selected {
                Style::default()
                    .fg(theme::BG)
                    .bg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!(" {:20}", name), style),
                Span::styled(format!(": {}", val), Style::default().fg(theme::TEXT_DIM)),
            ]))
        })
        .collect();

    f.render_widget(List::new(list_items).block(block), area);
}

fn draw_generic_prompt(f: &mut Frame, app: &App, size: Rect) {
    let w = 50.min(size.width.saturating_sub(4));
    let h = 3;
    let x = (size.width - w) / 2;
    let y = (size.height - h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let title = match app.settings_selected {
        0 => " Set Tab Size ",
        1 => " Set Autosave Interval (ms) ",
        _ => " Input ",
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .style(Style::default().bg(theme::OVERLAY_BG));

    let input = Paragraph::new(app.input_buffer.as_str()).block(block);
    f.render_widget(input, area);

    f.set_cursor_position((area.x + 1 + app.input_cursor as u16, area.y + 1));
}

// ─── AI Panel (Right Sidebar) ───────────────────────────────────────

fn draw_ai_panel(f: &mut Frame, app: &App, area: Rect) -> crate::events::AiPanelRect {
    let bc = if app.focus == crate::app::Focus::AiChat {
        theme::BORDER_ACTIVE
    } else {
        theme::BORDER
    };
    let title = if app.ai_state.is_streaming {
        let dots = match app.ai_state.stream_tick % 4 {
            0 => "●○○",
            1 => "○●○",
            2 => "○○●",
            _ => "●●●",
        };
        format!(" 🤖 AI Agent {} ", dots)
    } else {
        " 🤖 AI Agent ".to_string()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(bc))
        .style(Style::default().bg(theme::AI_BG));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if inner.width < 5 || inner.height < 4 {
        return crate::events::AiPanelRect {
            x: inner.x,
            y: inner.y,
            width: inner.width,
            height: inner.height,
        };
    }

    // Split: messages area + input area (3 lines)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // chat messages
            Constraint::Length(3), // input field
        ])
        .split(inner);

    let messages_area = chunks[0];
    let input_area = chunks[1];
    let content_w = messages_area.width as usize;

    // ── Build rendered lines from display messages ──
    let mut rendered_lines: Vec<Line<'static>> = Vec::new();

    if !app.config.ai.is_configured() {
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " ⚙ AI not configured",
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )));
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " Edit ~/.config/tcode/config.toml:",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " [ai]",
            Style::default().fg(theme::ACCENT),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " api_key = \"sk-...\"",
            Style::default().fg(theme::GREEN),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " base_url = \"https://api.openai.com/v1\"",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " model = \"gpt-4o-mini\"",
            Style::default().fg(theme::TEXT_DIM),
        )));
    } else if app.ai_state.display_messages.is_empty() && !app.ai_state.is_streaming {
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " 🤖 AI Agent ready",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )));
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " Tools: read_file, write_file,",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(Span::styled(
            "        run_command, list_directory",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(""));
        rendered_lines.push(Line::from(Span::styled(
            " Type a message and press Enter.",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " Ctrl+L: Insert last code block",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " Ctrl+D: Clear chat history",
            Style::default().fg(theme::TEXT_DIM),
        )));
        rendered_lines.push(Line::from(Span::styled(
            " Ctrl+E: Close AI panel",
            Style::default().fg(theme::TEXT_DIM),
        )));
    } else {
        for msg in &app.ai_state.display_messages {
            match msg {
                ai::DisplayMessage::User(text) => {
                    rendered_lines.push(Line::from(""));
                    rendered_lines.push(Line::from(Span::styled(
                        " 👤 You",
                        Style::default()
                            .fg(theme::ACCENT)
                            .add_modifier(Modifier::BOLD),
                    )));
                    for line in wrap_text(text, content_w.saturating_sub(2)) {
                        rendered_lines.push(Line::from(Span::styled(
                            format!(" {}", line),
                            Style::default().fg(theme::TEXT).bg(theme::AI_USER_BG),
                        )));
                    }
                }
                ai::DisplayMessage::Assistant(text) => {
                    rendered_lines.push(Line::from(""));
                    rendered_lines.push(Line::from(Span::styled(
                        " 🤖 Agent",
                        Style::default()
                            .fg(theme::MAUVE)
                            .add_modifier(Modifier::BOLD),
                    )));
                    render_markdown_lines(text, content_w.saturating_sub(2), &mut rendered_lines);
                }
                ai::DisplayMessage::ToolUse(exec) => {
                    let icon = if exec.success { "✓" } else { "✗" };
                    let color = if exec.success {
                        theme::GREEN
                    } else {
                        theme::RED
                    };
                    let tool_icon = match exec.tool_name.as_str() {
                        "read_file" => "📄",
                        "write_file" => "✏️",
                        "run_command" => "⚡",
                        "list_directory" => "📁",
                        _ => "🔧",
                    };
                    rendered_lines.push(Line::from(vec![
                        Span::styled(
                            format!(" {} {} ", tool_icon, exec.tool_name),
                            Style::default().fg(theme::MAUVE).bg(theme::AI_TOOL_BG),
                        ),
                        Span::styled(
                            format!("({})", truncate_display(&exec.arguments_summary, 25)),
                            Style::default().fg(theme::TEXT_DIM).bg(theme::AI_TOOL_BG),
                        ),
                    ]));
                    rendered_lines.push(Line::from(Span::styled(
                        format!("   {} {}", icon, exec.result_summary),
                        Style::default().fg(color),
                    )));
                }
            }
        }

        // Streaming buffer (current response being typed)
        if app.ai_state.is_streaming && !app.ai_state.streaming_buffer.is_empty() {
            rendered_lines.push(Line::from(""));
            rendered_lines.push(Line::from(Span::styled(
                " 🤖 Agent",
                Style::default()
                    .fg(theme::MAUVE)
                    .add_modifier(Modifier::BOLD),
            )));
            render_markdown_lines(
                &app.ai_state.streaming_buffer,
                content_w.saturating_sub(2),
                &mut rendered_lines,
            );
            // Blinking cursor
            rendered_lines.push(Line::from(Span::styled(
                " █",
                Style::default().fg(theme::MAUVE),
            )));
        } else if app.ai_state.is_streaming {
            rendered_lines.push(Line::from(""));
            let dots = match app.ai_state.stream_tick % 3 {
                0 => " ⏳ Thinking...",
                1 => " ⏳ Thinking..",
                _ => " ⏳ Thinking.",
            };
            rendered_lines.push(Line::from(Span::styled(
                dots,
                Style::default().fg(theme::YELLOW),
            )));
        }
    }

    // Apply scroll and render messages
    let vh = messages_area.height as usize;
    let total_lines = rendered_lines.len();
    let max_scroll = total_lines.saturating_sub(vh);
    let scroll = app.ai_state.scroll_offset.min(max_scroll);

    let visible: Vec<Line> = rendered_lines.into_iter().skip(scroll).take(vh).collect();

    f.render_widget(Paragraph::new(visible), messages_area);

    // ── Input field ──
    let input_block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme::BORDER))
        .style(Style::default().bg(theme::AI_BG));

    let input_inner = input_block.inner(input_area);
    f.render_widget(input_block, input_area);

    let prompt_text = if app.ai_state.is_streaming {
        " ⏳ Agent working...".to_string()
    } else {
        
        if app.ai_state.input_buffer.is_empty() {
            " Ask the AI agent...".to_string()
        } else {
            format!(" {}", app.ai_state.input_buffer)
        }
    };

    let input_style = if app.ai_state.input_buffer.is_empty() && !app.ai_state.is_streaming {
        Style::default().fg(theme::TEXT_DIM)
    } else {
        Style::default().fg(theme::TEXT)
    };

    f.render_widget(Paragraph::new(prompt_text).style(input_style), input_inner);

    // Cursor in input
    if app.focus == crate::app::Focus::AiChat && !app.ai_state.is_streaming {
        let cx = input_inner.x + 1 + app.ai_state.input_cursor as u16;
        let cy = input_inner.y;
        if cx < input_inner.right() && cy < input_inner.bottom() {
            f.set_cursor_position((cx, cy));
        }
    }

    crate::events::AiPanelRect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: inner.height,
    }
}

/// Render markdown-ish text with code block highlighting into line buffer.
fn render_markdown_lines(text: &str, max_w: usize, lines: &mut Vec<Line<'static>>) {
    let mut in_code_block = false;

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme::TEXT_DIM).bg(theme::AI_CODE_BG),
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    format!(" {}", line),
                    Style::default().fg(theme::TEXT_DIM).bg(theme::AI_CODE_BG),
                )));
            }
        } else if in_code_block {
            lines.push(Line::from(Span::styled(
                format!(" {}", line),
                Style::default().fg(theme::GREEN).bg(theme::AI_CODE_BG),
            )));
        } else {
            for wrapped in wrap_text(line, max_w) {
                lines.push(Line::from(Span::styled(
                    format!(" {}", wrapped),
                    Style::default().fg(theme::TEXT),
                )));
            }
        }
    }
}

/// Simple word-wrap for text.
fn wrap_text(text: &str, max_w: usize) -> Vec<String> {
    if max_w == 0 {
        return vec![text.to_string()];
    }
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_w {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            result.push(current_line);
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        result.push(current_line);
    }
    if result.is_empty() {
        result.push(String::new());
    }
    result
}

fn truncate_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

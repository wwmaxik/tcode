use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use std::time::Duration;

use crate::app::{App, InputMode};

#[derive(Default, Clone, Copy)]
pub struct TextAreaRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Default, Clone, Copy)]
pub struct ExplorerRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TabBarRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Default, Clone, Copy)]
pub struct ModalRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
    pub active: bool,
}

#[allow(dead_code)]
#[derive(Default, Clone, Copy)]
pub struct TerminalRect {
    pub width: u16,
    pub height: u16,
}

#[derive(Default, Clone, Copy)]
pub struct ActivityBarRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Default, Clone, Copy)]
#[allow(dead_code)]
pub struct AiPanelRect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

#[derive(Default, Clone, Copy)]
pub struct UiRects {
    pub text_area: TextAreaRect,
    pub explorer: ExplorerRect,
    pub tab_bar: TabBarRect,
    pub theme_modal: ModalRect,
    pub terminal: TerminalRect,
    pub activity_bar: ActivityBarRect,
    #[allow(dead_code)]
    pub ai_panel: AiPanelRect,
}

pub fn handle_events(app: &mut App, rects: UiRects) -> std::io::Result<bool> {
    if event::poll(Duration::from_millis(0))? {
        match event::read()? {
            Event::Key(key) => handle_key(app, key, rects),
            Event::Mouse(mouse) => handle_mouse(app, mouse, rects),
            _ => {}
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

fn handle_key(app: &mut App, key: KeyEvent, rects: UiRects) {
    app.status_msg = format!("Key: {:?} Mod: {:?}", key.code, key.modifiers);
    // ── Help overlay — any key closes ──
    if app.show_help {
        app.show_help = false;
        return;
    }

    // ── Plugin manager — Esc closes ──
    if app.show_plugin_manager {
        if key.code == KeyCode::Esc {
            app.show_plugin_manager = false;
        }
        return;
    }

    // ── Theme picker modal ──
    if app.show_theme_picker {
        handle_theme_picker_key(app, key, rects);
        return;
    }

    // ── Fuzzy Finder ──
    if app.show_fuzzy_finder {
        match key.code {
            KeyCode::Esc => app.toggle_fuzzy_finder(),
            KeyCode::Up => app.fuzzy_up(),
            KeyCode::Down => app.fuzzy_down(),
            KeyCode::Enter => app.fuzzy_enter(),
            KeyCode::Backspace => {
                app.fuzzy_query.pop();
                app.update_fuzzy_results();
            }
            KeyCode::Char(c) if c.is_alphanumeric() || c.is_ascii_punctuation() => {
                app.fuzzy_query.push(c);
                app.update_fuzzy_results();
            }
            _ => {}
        }
        return;
    }

    // ── Folder prompt ──
    if app.input_mode == InputMode::FolderPrompt {
        handle_prompt_key(app, key);
        return;
    }

    // ── Settings modal ──
    if app.show_settings && app.input_mode == InputMode::Editing {
        handle_settings_modal_key(app, key, rects);
        return;
    }

    // ── Generic prompt ──
    if app.input_mode == InputMode::Prompt {
        handle_generic_prompt_key(app, key);
        return;
    }

    fn handle_settings_modal_key(app: &mut App, key: KeyEvent, _rects: UiRects) {
        match key.code {
            KeyCode::Up => app.settings_prev(),
            KeyCode::Down => app.settings_next(),
            KeyCode::Enter => app.settings_enter(),
            KeyCode::Esc => app.show_settings = false,
            _ => {}
        }
    }

    fn handle_generic_prompt_key(app: &mut App, key: KeyEvent) {
        match key.code {
            KeyCode::Enter => app.confirm_settings_prompt(),
            KeyCode::Esc => app.cancel_settings_prompt(),
            KeyCode::Backspace => {
                if app.input_cursor > 0 {
                    app.input_cursor -= 1;
                    app.input_buffer.remove(app.input_cursor);
                }
            }
            KeyCode::Delete => {
                if app.input_cursor < app.input_buffer.len() {
                    app.input_buffer.remove(app.input_cursor);
                }
            }
            KeyCode::Left => {
                if app.input_cursor > 0 {
                    app.input_cursor -= 1;
                }
            }
            KeyCode::Right => {
                if app.input_cursor < app.input_buffer.len() {
                    app.input_cursor += 1;
                }
            }
            KeyCode::Char(ch) => {
                app.input_buffer.insert(app.input_cursor, ch);
                app.input_cursor += 1;
            }
            _ => {}
        }
    }

    // ── Ctrl combos ──
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('s') => app.save(),
            KeyCode::Char('b') => {
                if app.focus == crate::app::Focus::Explorer {
                    app.focus = crate::app::Focus::Editor;
                } else {
                    app.focus = crate::app::Focus::Explorer;
                }
            }
            KeyCode::Char('h') => app.toggle_help(),
            KeyCode::Char('o') => app.start_folder_prompt(),
            KeyCode::Char('w') => app.close_tab(),
            KeyCode::Char('n') => app.next_tab(),
            KeyCode::Char('p') => app.toggle_fuzzy_finder(),
            KeyCode::Char('c') => app.copy(),
            KeyCode::Char('v') => app.paste(),
            KeyCode::Char('`') | KeyCode::Char('ё') | KeyCode::Char('j') => {
                app.show_terminal = !app.show_terminal;
                if app.show_terminal {
                    app.focus = crate::app::Focus::Terminal;
                } else {
                    app.focus = crate::app::Focus::Editor;
                }
            }
            KeyCode::Char('t') => app.toggle_theme_picker(),
            KeyCode::Char('u') => app.toggle_plugin_manager(),
            KeyCode::Char('l') => app.open_settings(),
            KeyCode::Char('e') => app.toggle_ai_panel(),
            _ => {}
        }
        return;
    }

    // ── Alt combos ──
    if key.modifiers.contains(KeyModifiers::ALT) {
        match key.code {
            KeyCode::Right => app.next_tab(),
            KeyCode::Left => app.prev_tab(),
            KeyCode::Char('1') => {
                app.sidebar_panel = crate::app::SidebarPanel::Explorer;
                app.config.show_explorer = true;
            }
            KeyCode::Char('2') => {
                app.sidebar_panel = crate::app::SidebarPanel::Git;
                app.config.show_explorer = true;
                app.update_git_status();
            }
            KeyCode::Char('3') => {
                app.sidebar_panel = crate::app::SidebarPanel::Search;
                app.config.show_explorer = true;
            }
            KeyCode::Char('4') => {
                app.open_settings();
            }
            KeyCode::Char('5') => {
                app.toggle_help();
            }
            _ => {}
        }
        return;
    }

    // ── Explorer focused ──
    if app.focus == crate::app::Focus::Explorer {
        match key.code {
            KeyCode::Up => app.file_tree_up(),
            KeyCode::Down => app.file_tree_down(),
            KeyCode::Enter => app.file_tree_enter(),
            KeyCode::Esc => app.focus = crate::app::Focus::Editor,
            _ => {}
        }
        return;
    }

    // ── AI Chat focused ──
    if app.focus == crate::app::Focus::AiChat {
        // Ctrl combos within AI chat
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('l') => {
                    app.ai_insert_code();
                    return;
                }
                KeyCode::Char('d') => {
                    app.ai_clear_history();
                    return;
                }
                _ => {}
            }
        }
        match key.code {
            KeyCode::Enter => app.ai_send_message(),
            KeyCode::Esc => {
                app.focus = crate::app::Focus::Editor;
            }
            KeyCode::Backspace => app.ai_input_backspace(),
            KeyCode::Delete => app.ai_input_delete(),
            KeyCode::Left => app.ai_input_left(),
            KeyCode::Right => app.ai_input_right(),
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.ai_scroll_up();
                } else {
                    app.ai_scroll_up();
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    app.ai_scroll_down();
                } else {
                    app.ai_scroll_down();
                }
            }
            KeyCode::Char(ch) => app.ai_input_char(ch),
            _ => {}
        }
        return;
    }

    // ── Terminal focused ──
    if app.focus == crate::app::Focus::Terminal {
        if let Some(tx) = &app.pty_tx {
            let mut buf = Vec::new();
            match key.code {
                KeyCode::Up => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.terminal_scroll_up();
                        return;
                    } else {
                        buf.extend_from_slice(b"\x1b[A");
                    }
                }
                KeyCode::Down => {
                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                        app.terminal_scroll_down();
                        return;
                    } else {
                        buf.extend_from_slice(b"\x1b[B");
                    }
                }
                KeyCode::Char(c) => {
                    let mut b = [0; 4];
                    buf.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
                }
                KeyCode::Enter => buf.push(b'\n'),
                KeyCode::Backspace => buf.push(0x08),
                KeyCode::Tab => buf.push(b'\t'),
                KeyCode::Esc => buf.push(0x1b),
                KeyCode::Right => buf.extend_from_slice(b"\x1b[C"),
                KeyCode::Left => buf.extend_from_slice(b"\x1b[D"),
                _ => {}
            }
            if !buf.is_empty() {
                let _ = tx.send(crate::term::pty::PtyCommand::Data(buf));
            }
        }
        return;
    }

    // ── Autocomplete Menu ──
    if let Some(completions) = &app.lsp_completions {
        if !completions.is_empty() {
            match key.code {
                KeyCode::Up => {
                    if app.lsp_completion_selected > 0 {
                        app.lsp_completion_selected -= 1;
                    }
                    return;
                }
                KeyCode::Down => {
                    if app.lsp_completion_selected + 1 < completions.len() {
                        app.lsp_completion_selected += 1;
                    }
                    return;
                }
                KeyCode::Enter => {
                    app.apply_completion();
                    return;
                }
                KeyCode::Esc => {
                    app.lsp_completions = None;
                    return;
                }
                _ => {
                    // Let typing continue, but dismiss completions unless it's a typed char?
                    // Actually, if they type, we should just dismiss the old completions
                    // and wait for the new one from did_change.
                    app.lsp_completions = None;
                }
            }
        }
    }

    // ── Normal editor keys ──
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let tab_idx = app.active_tab;
    if app.tabs.len() > tab_idx {
        let t = &mut app.tabs[tab_idx];
        if has_shift && t.selection_start.is_none() {
            t.selection_start = Some((t.cursor_row, t.cursor_col));
        } else if !has_shift
            && matches!(
                key.code,
                KeyCode::Left
                    | KeyCode::Right
                    | KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::Home
                    | KeyCode::End
            )
        {
            t.selection_start = None;
        }
    }

    match key.code {
        KeyCode::Char(ch) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) && ch == 'a' {
                if let Some(t) = app.get_active_tab_mut() {
                    t.selection_start = Some((0, 0));
                    t.cursor_row = t.lines.len() - 1;
                    t.cursor_col = t.lines[t.cursor_row].len();
                }
            } else {
                app.insert_char(ch);
            }
        }
        KeyCode::Backspace => app.backspace(),
        KeyCode::Delete => app.delete_char(),
        KeyCode::Enter => app.enter(),
        KeyCode::Tab => app.tab_insert(),
        KeyCode::Left => app.move_left(),
        KeyCode::Right => app.move_right(),
        KeyCode::Up => app.move_up(),
        KeyCode::Down => app.move_down(),
        KeyCode::Home => app.home(),
        KeyCode::End => app.end(),
        KeyCode::Esc => app.focus = crate::app::Focus::Explorer,
        _ => {}
    }
}

fn handle_theme_picker_key(app: &mut App, key: KeyEvent, rects: UiRects) {
    let _ = rects; // available for future mouse coord use
    match key.code {
        KeyCode::Up => app.theme_up(),
        KeyCode::Down => app.theme_down(),
        KeyCode::Enter => app.theme_apply(),
        KeyCode::Esc => app.show_theme_picker = false,
        _ => {}
    }
}

fn handle_prompt_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => app.confirm_folder_prompt(),
        KeyCode::Esc => app.cancel_prompt(),
        KeyCode::Backspace => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
                app.input_buffer.remove(app.input_cursor);
            }
        }
        KeyCode::Left => {
            if app.input_cursor > 0 {
                app.input_cursor -= 1;
            }
        }
        KeyCode::Right => {
            if app.input_cursor < app.input_buffer.len() {
                app.input_cursor += 1;
            }
        }
        KeyCode::Char(ch) => {
            app.input_buffer.insert(app.input_cursor, ch);
            app.input_cursor += 1;
        }
        _ => {}
    }
}

fn handle_mouse(app: &mut App, mouse: MouseEvent, rects: UiRects) {
    let (col, row) = (mouse.column, mouse.row);

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // ── Click inside theme picker modal ──
            if app.show_theme_picker && rects.theme_modal.active {
                let m = rects.theme_modal;
                if col >= m.x && col < m.x + m.width && row >= m.y && row < m.y + m.height {
                    // +1 for the block border top
                    let rel_row = (row.saturating_sub(m.y + 1)) as usize;
                    app.theme_click(rel_row);
                    return;
                } else {
                    app.show_theme_picker = false;
                    return;
                }
            }

            // ── Click on plugin manager — dismiss ──
            if app.show_plugin_manager {
                app.show_plugin_manager = false;
                return;
            }

            // ── Click on tab bar ──
            if row >= rects.tab_bar.y && row < rects.tab_bar.y + rects.tab_bar.height {
                handle_tab_bar_click(app, col);
                return;
            }
            // ── Click on activity bar ──
            if col >= rects.activity_bar.x
                && col < rects.activity_bar.x + rects.activity_bar.width
                && row >= rects.activity_bar.y
                && row < rects.activity_bar.y + rects.activity_bar.height
            {
                let rel_row = (row - rects.activity_bar.y) as usize;
                match rel_row {
                    0..=2 => {
                        app.sidebar_panel = crate::app::SidebarPanel::Explorer;
                        app.config.show_explorer = true;
                    }
                    3..=5 => {
                        app.sidebar_panel = crate::app::SidebarPanel::Git;
                        app.config.show_explorer = true;
                        app.update_git_status();
                    }
                    6..=8 => {
                        app.sidebar_panel = crate::app::SidebarPanel::Search;
                        app.config.show_explorer = true;
                    }
                    9..=11 => {
                        app.sidebar_panel = crate::app::SidebarPanel::Plugins;
                        app.config.show_explorer = true;
                    }
                    r if r >= rects.activity_bar.height as usize - 6
                        && r < rects.activity_bar.height as usize - 3 =>
                    {
                        app.open_settings();
                    }
                    r if r >= rects.activity_bar.height as usize - 3 => {
                        app.toggle_help();
                    }
                    _ => {}
                }
                return;
            }
            // ── Click on explorer ──
            if col >= rects.explorer.x
                && col < rects.explorer.x + rects.explorer.width
                && row >= rects.explorer.y
                && row < rects.explorer.y + rects.explorer.height
            {
                let rel_row = (row - rects.explorer.y) as usize;
                let idx = app.file_tree_scroll + rel_row;
                app.file_tree_click(idx);
                return;
            }
            // ── Click on text area ──
            if col >= rects.text_area.x
                && col < rects.text_area.x + rects.text_area.width
                && row >= rects.text_area.y
                && row < rects.text_area.y + rects.text_area.height
            {
                let r = (row - rects.text_area.y) as usize;
                let c = (col - rects.text_area.x) as usize;
                app.mouse_click(r, c);
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if col >= rects.text_area.x
                && col < rects.text_area.x + rects.text_area.width
                && row >= rects.text_area.y
                && row < rects.text_area.y + rects.text_area.height
            {
                let r = (row - rects.text_area.y) as usize;
                let c = (col - rects.text_area.x) as usize;
                app.mouse_drag(r, c);
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            if let Some(t) = app.get_active_tab_mut() {
                if let Some(start) = t.selection_start {
                    if start == (t.cursor_row, t.cursor_col) {
                        t.selection_start = None;
                    }
                }
            }
        }

        // ── Middle-click on tab bar → close that tab ──
        MouseEventKind::Down(MouseButton::Middle) => {
            if row >= rects.tab_bar.y && row < rects.tab_bar.y + rects.tab_bar.height {
                if let Some(idx) = tab_index_at(app, col) {
                    app.close_tab_at(idx);
                }
            }
        }

        MouseEventKind::ScrollUp => app.scroll_up(3),
        MouseEventKind::ScrollDown => {
            let vh = rects.text_area.height as usize;
            app.scroll_down(3, vh);
        }
        _ => {}
    }
}

/// Find which tab index the mouse X position falls into.
fn tab_index_at(app: &App, click_col: u16) -> Option<usize> {
    let mut x: u16 = 0;
    for (i, tab) in app.tabs.iter().enumerate() {
        let w = (tab.name.len() + 4) as u16;
        if click_col >= x && click_col < x + w {
            return Some(i);
        }
        x += w + 1;
    }
    None
}

fn handle_tab_bar_click(app: &mut App, click_col: u16) {
    if let Some(idx) = tab_index_at(app, click_col) {
        app.switch_to_tab(idx);
    }
}

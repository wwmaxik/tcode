use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum SidebarPanel {
    Explorer,
    Git,
    Search,
    Plugins,
}

pub struct GitChange {
    pub path: String,
    pub status: String,
}

#[derive(PartialEq, Eq)]
pub enum InputMode {
    Editing,
    FolderPrompt,
    Prompt,
}

pub struct Tab {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_offset: usize,
    pub scroll_x: usize,
    pub dirty: bool,
    pub file_path: Option<PathBuf>,
    pub name: String,
    /// Incremented on every text edit; used for highlight cache invalidation.
    pub text_version: u64,
    pub git_marks: Vec<Option<char>>,
    pub selection_start: Option<(usize, usize)>, // (row, col)
}

impl Tab {
    pub fn welcome() -> Self {
        Self {
            lines: vec![
                "// Welcome to tcode — a VS Code-like TUI editor".into(),
                "// Press Ctrl+H for keyboard shortcuts.".into(),
                "".into(),
                "fn main() {".into(),
                "    println!(\"Hello, tcode!\");".into(),
                "}".into(),
                "".into(),
            ],
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            scroll_x: 0,
            dirty: false,
            file_path: None,
            name: "welcome".into(),
            text_version: 0,
            git_marks: Vec::new(),
            selection_start: None,
        }
    }

    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("{}", e))?;
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "untitled".into());
        Ok(Self {
            lines,
            cursor_row: 0,
            cursor_col: 0,
            scroll_offset: 0,
            scroll_x: 0,
            dirty: false,
            file_path: Some(path.to_path_buf()),
            name,
            text_version: 0,
            git_marks: Vec::new(),
            selection_start: None,
        })
    }

    pub fn update_git_marks(&mut self, repo: Option<&git2::Repository>, cwd: &Path) {
        if let Some(repo) = repo
            && let Some(path) = &self.file_path
            && let Ok(rel_path) = path.strip_prefix(cwd)
        {
            let mut original_content = Vec::new();
            if let Ok(head) = repo.head()
                && let Ok(tree) = head.peel_to_tree()
                && let Ok(entry) = tree.get_path(rel_path)
                && let Ok(obj) = entry.to_object(repo)
                && let Some(blob) = obj.as_blob()
            {
                original_content = blob.content().to_vec();
            }

            let current_content = self.lines.join("\n").into_bytes();
            if let Ok(patch) =
                git2::Patch::from_buffers(&original_content, None, &current_content, None, None)
            {
                let mut marks = vec![None; self.lines.len()];
                for h in 0..patch.num_hunks() {
                    if let Ok((hunk, _)) = patch.hunk(h) {
                        let old_lines = hunk.old_lines();
                        let new_lines = hunk.new_lines();
                        let new_start = hunk.new_start() as usize;

                        if old_lines > 0 && new_lines > 0 {
                            for i in 0..new_lines as usize {
                                if new_start + i > 0 && new_start + i - 1 < marks.len() {
                                    marks[new_start + i - 1] = Some('~');
                                }
                            }
                        } else if old_lines == 0 && new_lines > 0 {
                            for i in 0..new_lines as usize {
                                if new_start + i > 0 && new_start + i - 1 < marks.len() {
                                    marks[new_start + i - 1] = Some('+');
                                }
                            }
                        } else if new_lines == 0 && old_lines > 0 {
                            if new_start > 0 && new_start - 1 < marks.len() {
                                marks[new_start - 1] = Some('|');
                            } else if !marks.is_empty() {
                                marks[0] = Some('|');
                            }
                        }
                    }
                }
                self.git_marks = marks;
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum Focus {
    Explorer,
    Editor,
    Terminal,
    AiChat,
}

pub struct App {
    pub should_quit: bool,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
    pub status_msg: String,
    pub file_tree: Vec<FileEntry>,
    pub file_tree_selected: usize,
    pub file_tree_scroll: usize,
    pub focus: Focus,
    pub cwd: PathBuf,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub show_help: bool,
    /// Set to true after a successful save; main loop uses this to fire plugin on_save hooks.
    pub just_saved: bool,
    // ── Theme system ──
    pub current_theme: String,
    pub show_theme_picker: bool,
    pub theme_list: Vec<String>,
    pub theme_selected: usize,
    // ── Plugin manager ──
    pub show_plugin_manager: bool,
    pub plugin_info: Vec<(String, bool)>,
    // ── Terminal ──
    pub show_terminal: bool,
    pub terminal: crate::term::TerminalState,
    pub terminal_parser: vte::Parser,
    pub pty_tx: Option<std::sync::mpsc::Sender<crate::term::pty::PtyCommand>>,
    // ── LSP ──
    pub lsp_client: Option<crate::lsp::LspClient>,
    pub lsp_initialized: bool,
    pub lsp_diagnostics: Vec<lsp_types::Diagnostic>,
    pub lsp_completions: Option<Vec<lsp_types::CompletionItem>>,
    pub lsp_completion_selected: usize,
    // ── New IDE Features ──
    pub config: crate::config::Config,
    pub clipboard: Option<arboard::Clipboard>,
    pub git_repo: Option<git2::Repository>,
    pub show_fuzzy_finder: bool,
    pub fuzzy_query: String,
    pub fuzzy_results: Vec<String>,
    pub fuzzy_selected: usize,

    pub show_settings: bool,
    pub settings_selected: usize,

    // ── Sidebar Panels ──
    pub sidebar_panel: SidebarPanel,
    pub git_changes: Vec<GitChange>,
    pub git_selected: usize,

    // ── AI Agent ──
    pub show_ai_panel: bool,
    pub ai_state: crate::ai::AiState,
    pub ai_event_rx: Option<std::sync::mpsc::Receiver<crate::ai::AiEvent>>,
}

impl App {
    pub fn new(cwd: PathBuf) -> Self {
        let config = crate::config::Config::load();
        let session = crate::config::Session::load();
        let file_tree = read_directory(&cwd);
        let clipboard = arboard::Clipboard::new().ok();
        let git_repo = git2::Repository::discover(&cwd).ok();

        let mut tabs = Vec::new();
        for file in &session.open_files {
            if let Ok(tab) = Tab::from_file(Path::new(file)) {
                tabs.push(tab);
            }
        }
        if tabs.is_empty() {
            tabs.push(Tab::welcome());
        }
        let active_tab = session.active_file_index.min(tabs.len().saturating_sub(1));

        Self {
            should_quit: false,
            tabs,
            active_tab,
            status_msg: "Ready — Ctrl+H for help".into(),
            file_tree,
            file_tree_selected: 0,
            file_tree_scroll: 0,
            focus: if config.show_explorer {
                Focus::Explorer
            } else {
                Focus::Editor
            },
            cwd,
            input_mode: InputMode::Editing,
            input_buffer: String::new(),
            input_cursor: 0,
            show_help: false,
            just_saved: false,
            show_theme_picker: false,
            theme_list: Vec::new(),
            theme_selected: 0,
            show_plugin_manager: false,
            plugin_info: Vec::new(),
            show_terminal: false,
            terminal: crate::term::TerminalState::new(80, 24),
            terminal_parser: vte::Parser::new(),
            pty_tx: None,
            lsp_client: None,
            lsp_initialized: false,
            lsp_diagnostics: Vec::new(),
            lsp_completions: None,
            lsp_completion_selected: 0,
            current_theme: config.theme.clone(),
            config,
            clipboard,
            git_repo,
            show_fuzzy_finder: false,
            fuzzy_query: String::new(),
            fuzzy_results: Vec::new(),
            fuzzy_selected: 0,
            show_settings: false,
            settings_selected: 0,
            sidebar_panel: SidebarPanel::Explorer,
            git_changes: Vec::new(),
            git_selected: 0,
            // AI
            show_ai_panel: false,
            ai_state: crate::ai::AiState::default(),
            ai_event_rx: None,
        }
    }

    pub fn update_git_status(&mut self) {
        if let Some(repo) = &self.git_repo {
            let mut changes = Vec::new();
            if let Ok(statuses) = repo.statuses(None) {
                for entry in statuses.iter() {
                    let status = entry.status();
                    let path = entry.path().unwrap_or("unknown").to_string();
                    let marker = if status.is_index_new() || status.is_wt_new() {
                        "A"
                    } else if status.is_index_modified() || status.is_wt_modified() {
                        "M"
                    } else if status.is_index_deleted() || status.is_wt_deleted() {
                        "D"
                    } else if status.is_index_renamed() || status.is_wt_renamed() {
                        "R"
                    } else if status.is_index_typechange() || status.is_wt_typechange() {
                        "T"
                    } else {
                        "?"
                    };
                    changes.push(GitChange {
                        path,
                        status: marker.to_string(),
                    });
                }
            }
            self.git_changes = changes;
        }
    }

    // ── Session & Saving ────────────────────────────────────────

    pub fn save_session(&self) {
        let open_files = self
            .tabs
            .iter()
            .filter_map(|t| {
                t.file_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
            })
            .collect();
        let session = crate::config::Session {
            open_files,
            active_file_index: self.active_tab,
        };
        session.save();
    }

    pub fn save_all(&mut self) {
        for tab in &mut self.tabs {
            if tab.dirty
                && let Some(path) = &tab.file_path
            {
                let content = tab.lines.join("\n");
                if std::fs::write(path, content).is_ok() {
                    tab.dirty = false;
                    self.just_saved = true;
                }
            }
        }
    }

    pub fn sync_lsp_active_tab(&self) {
        if !self.lsp_initialized {
            return;
        }
        if let Some(client) = &self.lsp_client {
            let tab = &self.tabs[self.active_tab];
            if let Some(path) = &tab.file_path {
                let abs_path = if path.is_absolute() {
                    path.clone()
                } else {
                    self.cwd.join(path)
                };
                if let Ok(url) = url::Url::from_file_path(abs_path) {
                    client.did_change(url, tab.lines.join("\n"), tab.text_version as i32);
                }
            }
        }
    }

    pub fn trigger_completion(&mut self) {
        if !self.lsp_initialized {
            return;
        }
        if let Some(client) = &mut self.lsp_client {
            let tab = &self.tabs[self.active_tab];
            if let Some(path) = &tab.file_path {
                let abs_path = if path.is_absolute() {
                    path.clone()
                } else {
                    self.cwd.join(path)
                };
                if let Ok(url) = url::Url::from_file_path(abs_path) {
                    client.completion(url, tab.cursor_row as u32, tab.cursor_col as u32);
                }
            }
        }
    }

    pub fn apply_completion(&mut self) {
        if let Some(completions) = self.lsp_completions.take()
            && self.lsp_completion_selected < completions.len()
        {
            let item = &completions[self.lsp_completion_selected];
            let text_to_insert = item
                .insert_text
                .clone()
                .unwrap_or_else(|| item.label.clone());

            let t = &mut self.tabs[self.active_tab];
            // Simple MVP: backspace until we hit a non-alphanumeric char
            while t.cursor_col > 0 {
                let prev_idx = t.lines[t.cursor_row][..t.cursor_col]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let ch = t.lines[t.cursor_row][prev_idx..].chars().next().unwrap();
                if ch.is_alphanumeric() || ch == '_' {
                    t.lines[t.cursor_row].remove(prev_idx);
                    t.cursor_col = prev_idx;
                } else {
                    break;
                }
            }

            for ch in text_to_insert.chars() {
                t.lines[t.cursor_row].insert(t.cursor_col, ch);
                t.cursor_col += ch.len_utf8();
            }
            t.dirty = true;
            t.text_version += 1;
            self.sync_lsp_active_tab();
        }
        self.lsp_completion_selected = 0;
    }

    // ── Text editing ────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col > t.lines[t.cursor_row].len() {
            t.cursor_col = t.lines[t.cursor_row].len();
        }
        t.lines[t.cursor_row].insert(t.cursor_col, ch);
        t.cursor_col += ch.len_utf8();
        t.dirty = true;
        t.text_version += 1;
        self.sync_lsp_active_tab();
        self.trigger_completion();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col > 0 {
            let prev = t.lines[t.cursor_row][..t.cursor_col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            t.lines[t.cursor_row].remove(prev);
            t.cursor_col = prev;
            t.dirty = true;
            t.text_version += 1;
            self.sync_lsp_active_tab();
            self.trigger_completion();
        } else if t.cursor_row > 0 {
            let cur = t.lines.remove(t.cursor_row);
            t.cursor_row -= 1;
            t.cursor_col = t.lines[t.cursor_row].len();
            t.lines[t.cursor_row].push_str(&cur);
            t.dirty = true;
            t.text_version += 1;
            self.sync_lsp_active_tab();
            self.trigger_completion();
        }
    }

    pub fn delete_char(&mut self) {
        if self.delete_selection() {
            return;
        }
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col < t.lines[t.cursor_row].len() {
            t.lines[t.cursor_row].remove(t.cursor_col);
            t.dirty = true;
            t.text_version += 1;
            self.sync_lsp_active_tab();
            self.trigger_completion();
        } else if t.cursor_row + 1 < t.lines.len() {
            let next = t.lines.remove(t.cursor_row + 1);
            t.lines[t.cursor_row].push_str(&next);
            t.dirty = true;
            t.text_version += 1;
            self.sync_lsp_active_tab();
            self.trigger_completion();
        }
    }

    pub fn enter(&mut self) {
        self.delete_selection();
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col > t.lines[t.cursor_row].len() {
            t.cursor_col = t.lines[t.cursor_row].len();
        }
        let rem = t.lines[t.cursor_row][t.cursor_col..].to_string();
        t.lines[t.cursor_row].truncate(t.cursor_col);
        t.cursor_row += 1;
        t.lines.insert(t.cursor_row, rem);
        t.cursor_col = 0;
        t.dirty = true;
        t.text_version += 1;
        self.sync_lsp_active_tab();
        self.trigger_completion();
    }

    pub fn tab_insert(&mut self) {
        for _ in 0..4 {
            self.insert_char(' ');
        }
    }

    // ── Clipboard & Fuzzy Finder ────────────────────────────────

    pub fn get_selection_text(&self) -> Option<String> {
        let tab = self.get_active_tab()?;
        let start = tab.selection_start?;
        let end = (tab.cursor_row, tab.cursor_col);
        let (r1, c1, r2, c2) = if start < end {
            (start.0, start.1, end.0, end.1)
        } else {
            (end.0, end.1, start.0, start.1)
        };

        if r1 == r2 {
            Some(tab.lines[r1][c1..c2].to_string())
        } else {
            let mut result = Vec::new();
            result.push(tab.lines[r1][c1..].to_string());
            for r in r1 + 1..r2 {
                result.push(tab.lines[r].clone());
            }
            result.push(tab.lines[r2][..c2.min(tab.lines[r2].len())].to_string());
            Some(result.join("\n"))
        }
    }

    pub fn copy(&mut self) {
        let text = if let Some(sel) = self.get_selection_text() {
            sel
        } else {
            let tab = match self.get_active_tab() {
                Some(t) => t,
                None => return,
            };
            tab.lines[tab.cursor_row].clone()
        };
        if let Some(cb) = &mut self.clipboard {
            let _ = cb.set_text(text);
        }
    }

    pub fn paste(&mut self) {
        if self.tabs.is_empty() {
            return;
        }
        let text = if let Some(cb) = &mut self.clipboard {
            cb.get_text().unwrap_or_default()
        } else {
            return;
        };
        if text.is_empty() {
            return;
        }

        self.delete_selection();

        for ch in text.chars() {
            if ch == '\n' {
                self.enter();
            } else if ch != '\r' {
                self.insert_char(ch);
            }
        }
    }

    pub fn toggle_fuzzy_finder(&mut self) {
        self.show_fuzzy_finder = !self.show_fuzzy_finder;
        if self.show_fuzzy_finder {
            self.fuzzy_query.clear();
            self.fuzzy_results.clear();
            self.fuzzy_selected = 0;
            self.update_fuzzy_results();
        } else {
            self.focus = crate::app::Focus::Editor;
        }
    }

    pub fn update_fuzzy_results(&mut self) {
        let files = crate::fuzzy::scan_directory(&self.cwd);
        if self.fuzzy_query.is_empty() {
            self.fuzzy_results = files.into_iter().take(50).collect();
        } else {
            self.fuzzy_results = crate::fuzzy::match_files(&self.fuzzy_query, files);
        }
        self.fuzzy_selected = 0;
    }

    pub fn fuzzy_up(&mut self) {
        if self.fuzzy_selected > 0 {
            self.fuzzy_selected -= 1;
        }
    }

    pub fn fuzzy_down(&mut self) {
        if self.fuzzy_selected + 1 < self.fuzzy_results.len() {
            self.fuzzy_selected += 1;
        }
    }

    pub fn fuzzy_enter(&mut self) {
        if self.fuzzy_selected < self.fuzzy_results.len() {
            let file = &self.fuzzy_results[self.fuzzy_selected].clone();
            let path = self.cwd.join(file);
            self.open_file_path(&path);
        }
        self.show_fuzzy_finder = false;
        self.focus = crate::app::Focus::Editor;
    }

    pub fn open_file_path(&mut self, path: &Path) {
        if let Ok(tab) = Tab::from_file(path) {
            self.tabs.push(tab);
            self.active_tab = self.tabs.len() - 1;
        }
    }

    pub fn open_settings(&mut self) {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.settings_selected = 0;
            self.focus = crate::app::Focus::Editor; // or maybe a specific focus? Editor is fine if we intercept keys.
        }
    }

    pub fn settings_next(&mut self) {
        if self.settings_selected + 1 < 4 {
            // We have 4 settings items
            self.settings_selected += 1;
        }
    }

    pub fn settings_prev(&mut self) {
        if self.settings_selected > 0 {
            self.settings_selected -= 1;
        }
    }

    pub fn settings_enter(&mut self) {
        match self.settings_selected {
            0 => {
                // Tab Size
                self.input_mode = InputMode::Prompt;
                self.input_buffer = self.config.tab_size.to_string();
                self.input_cursor = self.input_buffer.len();
                // We'll parse this in prompt_result
            }
            1 => {
                // Autosave
                self.input_mode = InputMode::Prompt;
                self.input_buffer = self.config.autosave_interval_ms.to_string();
                self.input_cursor = self.input_buffer.len();
            }
            2 => {
                // Show Explorer
                self.config.show_explorer = !self.config.show_explorer;
                self.config.save();
                self.focus = if self.config.show_explorer {
                    Focus::Explorer
                } else {
                    Focus::Editor
                };
            }
            3 => {
                // Theme
                self.show_settings = false;
                self.toggle_theme_picker();
            }
            _ => {}
        }
    }

    pub fn confirm_settings_prompt(&mut self) {
        let input = self.input_buffer.clone();
        match self.settings_selected {
            0 => {
                // Tab Size
                if let Ok(val) = input.parse::<u16>() {
                    self.config.tab_size = val;
                    self.config.save();
                }
            }
            1 => {
                // Autosave Interval
                if let Ok(val) = input.parse::<u64>() {
                    self.config.autosave_interval_ms = val;
                    self.config.save();
                }
            }
            _ => {}
        }
        self.input_mode = InputMode::Editing;
    }

    pub fn cancel_settings_prompt(&mut self) {
        self.input_mode = InputMode::Editing;
    }

    // ── Cursor movement ─────────────────────────────────────────

    pub fn move_left(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col > 0 {
            let prev = t.lines[t.cursor_row][..t.cursor_col]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            t.cursor_col = prev;
        } else if t.cursor_row > 0 {
            t.cursor_row -= 1;
            t.cursor_col = t.lines[t.cursor_row].len();
        }
    }

    pub fn move_right(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_col < t.lines[t.cursor_row].len() {
            let ch = t.lines[t.cursor_row][t.cursor_col..]
                .chars()
                .next()
                .unwrap();
            t.cursor_col += ch.len_utf8();
        } else if t.cursor_row + 1 < t.lines.len() {
            t.cursor_row += 1;
            t.cursor_col = 0;
        }
    }

    pub fn move_up(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_row > 0 {
            t.cursor_row -= 1;
            let len = t.lines[t.cursor_row].len();
            if t.cursor_col > len {
                t.cursor_col = len;
            }
        }
    }

    pub fn move_down(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        if t.cursor_row + 1 < t.lines.len() {
            t.cursor_row += 1;
            let len = t.lines[t.cursor_row].len();
            if t.cursor_col > len {
                t.cursor_col = len;
            }
        }
    }

    pub fn home(&mut self) {
        self.tabs[self.active_tab].cursor_col = 0;
    }

    pub fn end(&mut self) {
        let t = &mut self.tabs[self.active_tab];
        t.cursor_col = t.lines[t.cursor_row].len();
    }

    // ── Mouse ───────────────────────────────────────────────────

    pub fn mouse_click(&mut self, row: usize, col: usize) {
        self.focus = crate::app::Focus::Editor;
        let tab_idx = self.active_tab;
        if self.tabs.len() > tab_idx {
            let t = &mut self.tabs[tab_idx];
            t.cursor_row = (row + t.scroll_offset).min(t.lines.len().saturating_sub(1));
            t.cursor_col = (col + t.scroll_x).min(t.lines[t.cursor_row].len());
            t.selection_start = Some((t.cursor_row, t.cursor_col));
        }
    }

    pub fn mouse_drag(&mut self, row: usize, col: usize) {
        let tab_idx = self.active_tab;
        if self.tabs.len() > tab_idx {
            let t = &mut self.tabs[tab_idx];
            t.cursor_row = (row + t.scroll_offset).min(t.lines.len().saturating_sub(1));
            t.cursor_col = (col + t.scroll_x).min(t.lines[t.cursor_row].len());
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.tabs[self.active_tab].scroll_offset =
            self.tabs[self.active_tab].scroll_offset.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize, vh: usize) {
        let t = &mut self.tabs[self.active_tab];
        if vh == 0 {
            return;
        }
        let max = t.lines.len().saturating_sub(1); // can scroll until last line is visible
        t.scroll_offset = (t.scroll_offset + n).min(max);
    }

    pub fn ensure_cursor_visible(&mut self, vh: usize) {
        if vh == 0 {
            return;
        }
        let t = &mut self.tabs[self.active_tab];
        // Keep a margin of 3 lines from top/bottom edges when possible
        let margin = 3.min(vh / 2);
        if t.cursor_row < t.scroll_offset + margin {
            t.scroll_offset = t.cursor_row.saturating_sub(margin);
        } else if t.cursor_row + margin >= t.scroll_offset + vh {
            t.scroll_offset = (t.cursor_row + margin + 1).saturating_sub(vh);
        }
        // Clamp scroll_offset so we don't scroll past the end
        let max_scroll = t.lines.len().saturating_sub(vh);
        if t.scroll_offset > max_scroll {
            t.scroll_offset = max_scroll;
        }
    }

    // ── Save ────────────────────────────────────────────────────

    pub fn save(&mut self) {
        let idx = self.active_tab;
        let has_path = self.tabs[idx].file_path.is_some();
        if has_path {
            let content = self.tabs[idx].lines.join("\n");
            let path = self.tabs[idx].file_path.as_ref().unwrap().clone();
            match fs::write(&path, &content) {
                Ok(()) => {
                    self.tabs[idx].dirty = false;
                    self.status_msg = format!("\"{}\" saved", self.tabs[idx].name);
                    self.just_saved = true;
                    self.update_git_status();
                }
                Err(e) => self.status_msg = format!("Error: {}", e),
            }
        } else {
            self.status_msg = "No file path".into();
        }
    }

    // ── Tab management ──────────────────────────────────────────

    pub fn next_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.tabs.len() > 1 {
            self.active_tab = if self.active_tab == 0 {
                self.tabs.len() - 1
            } else {
                self.active_tab - 1
            };
        }
    }

    pub fn get_active_tab(&self) -> Option<&Tab> {
        self.tabs.get(self.active_tab)
    }

    pub fn get_active_tab_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn delete_selection(&mut self) -> bool {
        let tab = match self.get_active_tab_mut() {
            Some(t) => t,
            None => return false,
        };
        let start = match tab.selection_start {
            Some(s) => s,
            None => return false,
        };
        let end = (tab.cursor_row, tab.cursor_col);

        let (r1, c1, r2, c2) = if start < end {
            (start.0, start.1, end.0, end.1)
        } else {
            (end.0, end.1, start.0, start.1)
        };

        if r1 == r2 {
            let line = &mut tab.lines[r1];
            if c1 < line.len() && c2 <= line.len() {
                line.replace_range(c1..c2, "");
            }
        } else {
            // Multiline delete
            let first_part = tab.lines[r1][..c1].to_string();
            let last_part = &tab.lines[r2][c2.min(tab.lines[r2].len())..];

            tab.lines[r1] = first_part + last_part;
            for _ in 0..(r2 - r1) {
                tab.lines.remove(r1 + 1);
            }
        }

        tab.cursor_row = r1;
        tab.cursor_col = c1;
        tab.selection_start = None;
        tab.dirty = true;
        tab.text_version += 1;
        true
    }

    pub fn close_tab(&mut self) {
        self.close_tab_at(self.active_tab);
    }

    pub fn close_tab_at(&mut self, idx: usize) {
        if self.tabs.len() > 1 && idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            } else if self.active_tab > idx {
                self.active_tab -= 1;
            }
        } else if self.tabs.len() <= 1 {
            self.status_msg = "Cannot close last tab".into();
        }
    }

    pub fn switch_to_tab(&mut self, idx: usize) {
        if idx < self.tabs.len() {
            self.active_tab = idx;
        }
    }

    pub fn open_file_from_path(&mut self, path: &Path) {
        for (i, tab) in self.tabs.iter().enumerate() {
            if tab.file_path.as_deref() == Some(path) {
                self.active_tab = i;
                self.status_msg = format!("Switched to {}", tab.name);
                return;
            }
        }
        match Tab::from_file(path) {
            Ok(tab) => {
                let name = tab.name.clone();
                self.tabs.push(tab);
                self.active_tab = self.tabs.len() - 1;
                self.status_msg = format!("Opened {}", name);
            }
            Err(e) => self.status_msg = format!("Error: {}", e),
        }
    }

    pub fn terminal_scroll_up(&mut self) {
        let max_scroll = self.terminal.lines.len().saturating_sub(2);
        if self.terminal.scroll_offset < max_scroll {
            self.terminal.scroll_offset += 1;
        }
    }

    pub fn terminal_scroll_down(&mut self) {
        self.terminal.scroll_offset = self.terminal.scroll_offset.saturating_sub(1);
    }

    // ── File tree ───────────────────────────────────────────────

    pub fn open_folder(&mut self, path: &Path) {
        let p = if path.is_relative() {
            self.cwd.join(path)
        } else {
            path.to_path_buf()
        };
        if p.is_dir() {
            self.cwd = p.clone();
            self.file_tree = read_directory(&p);
            self.file_tree_selected = 0;
            self.file_tree_scroll = 0;
            self.status_msg = format!("Folder: {}", self.cwd.display());
        } else {
            self.status_msg = format!("Not a directory: {}", p.display());
        }
    }

    pub fn file_tree_up(&mut self) {
        if self.file_tree_selected > 0 {
            self.file_tree_selected -= 1;
        }
    }

    pub fn file_tree_down(&mut self) {
        if self.file_tree_selected + 1 < self.file_tree.len() {
            self.file_tree_selected += 1;
        }
    }

    pub fn file_tree_enter(&mut self) {
        if let Some(entry) = self.file_tree.get(self.file_tree_selected).cloned() {
            if entry.is_dir {
                self.open_folder(&entry.path);
            } else {
                self.open_file_from_path(&entry.path);
                self.focus = Focus::Editor;
            }
        }
    }

    pub fn file_tree_click(&mut self, index: usize) {
        if index < self.file_tree.len() {
            if self.file_tree_selected == index && self.focus == Focus::Explorer {
                // Second click on same item — open it
                self.file_tree_enter();
            } else {
                self.file_tree_selected = index;
                self.focus = Focus::Explorer;
            }
        }
    }

    // ── Prompt ───────────────────────────────────────────────────

    pub fn start_folder_prompt(&mut self) {
        self.input_mode = InputMode::FolderPrompt;
        self.input_buffer = self.cwd.to_string_lossy().to_string();
        self.input_cursor = self.input_buffer.len();
    }

    pub fn confirm_folder_prompt(&mut self) {
        let path = PathBuf::from(&self.input_buffer);
        self.input_mode = InputMode::Editing;
        self.input_buffer.clear();
        self.input_cursor = 0;
        self.open_folder(&path);
    }

    pub fn cancel_prompt(&mut self) {
        self.input_mode = InputMode::Editing;
        self.input_buffer.clear();
        self.input_cursor = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    // ── Theme picker ────────────────────────────────────────────

    pub fn toggle_theme_picker(&mut self) {
        self.show_theme_picker = !self.show_theme_picker;
        if self.show_theme_picker {
            // Find current theme in list
            self.theme_selected = self
                .theme_list
                .iter()
                .position(|t| t == &self.current_theme)
                .unwrap_or(0);
        }
    }

    pub fn theme_up(&mut self) {
        if self.theme_selected > 0 {
            self.theme_selected -= 1;
        }
    }

    pub fn theme_down(&mut self) {
        if self.theme_selected + 1 < self.theme_list.len() {
            self.theme_selected += 1;
        }
    }

    pub fn theme_apply(&mut self) {
        if let Some(name) = self.theme_list.get(self.theme_selected) {
            self.current_theme = name.clone();
            self.status_msg = format!("Theme: {}", name);
            self.show_theme_picker = false;
        }
    }

    pub fn theme_click(&mut self, index: usize) {
        if index < self.theme_list.len() {
            self.theme_selected = index;
            self.theme_apply();
        }
    }

    // ── Plugin manager ──────────────────────────────────────────

    pub fn toggle_plugin_manager(&mut self) {
        self.show_plugin_manager = !self.show_plugin_manager;
        if self.show_plugin_manager {
            self.refresh_plugin_info();
        }
    }

    pub fn refresh_plugin_info(&mut self) {
        self.plugin_info.clear();
        let dir = self.cwd.join("plugins");
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) == Some("rhai") {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.plugin_info.push((name, true)); // true = loaded
                }
            }
            self.plugin_info.sort_by(|a, b| a.0.cmp(&b.0));
        }
    }

    // ── AI Agent ─────────────────────────────────────────────────

    pub fn toggle_ai_panel(&mut self) {
        self.show_ai_panel = !self.show_ai_panel;
        if self.show_ai_panel {
            self.focus = Focus::AiChat;
        } else if self.focus == Focus::AiChat {
            self.focus = Focus::Editor;
        }
    }

    pub fn ai_send_message(&mut self) {
        let input = self.ai_state.input_buffer.trim().to_string();
        if input.is_empty() || self.ai_state.is_streaming {
            return;
        }

        if !self.config.ai.is_configured() {
            self.status_msg = "AI: Set api_key in ~/.config/tcode/config.toml [ai] section".into();
            return;
        }

        // Build context from current file
        let context = self.build_ai_context();
        let user_content = if context.is_empty() {
            input.clone()
        } else {
            format!("{}\n\n{}", context, input)
        };

        // Add to display messages (show only the user's text, not the context)
        self.ai_state
            .display_messages
            .push(crate::ai::DisplayMessage::User(input.clone()));
        // Add to API messages (with context)
        self.ai_state
            .api_messages
            .push(crate::ai::ChatMessage::user(&user_content));

        self.ai_state.input_buffer.clear();
        self.ai_state.input_cursor = 0;
        self.ai_state.is_streaming = true;
        self.ai_state.streaming_buffer.clear();
        self.ai_state.stream_tick = 0;

        // Build full API message list with system prompt
        let mut full_messages = vec![crate::ai::ChatMessage::system(
            &self.config.ai.system_prompt,
        )];
        // Add conversation history (keep last ~40 API messages for context)
        let history_start = self.ai_state.api_messages.len().saturating_sub(40);
        for msg in &self.ai_state.api_messages[history_start..] {
            full_messages.push(msg.clone());
        }

        // Launch agent
        let (tx, rx) = std::sync::mpsc::channel();
        self.ai_event_rx = Some(rx);
        let cwd = self.cwd.clone();
        crate::ai::run_agent(&self.config.ai, full_messages, cwd, tx);

        self.status_msg = "AI Agent: Working...".into();
    }

    fn build_ai_context(&self) -> String {
        if self.tabs.is_empty() {
            return String::new();
        }
        let tab = &self.tabs[self.active_tab];
        let file_name = &tab.name;
        let ext = tab
            .file_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str())
            .unwrap_or("txt");

        // If there's a selection, use that as context
        if let Some(sel_text) = self.get_selection_text() {
            return format!(
                "[Current file: {} ({})]\nSelected code:\n```{}\n{}\n```",
                file_name, ext, ext, sel_text
            );
        }

        // Otherwise provide the whole file (truncated to ~200 lines)
        let max_lines = 200.min(tab.lines.len());
        let content: String = tab.lines[..max_lines].join("\n");
        let truncated = if tab.lines.len() > max_lines {
            format!("\n... ({} more lines)", tab.lines.len() - max_lines)
        } else {
            String::new()
        };

        format!(
            "[Current file: {} ({}) — cursor at line {}, col {}]\n```{}\n{}{}\n```",
            file_name,
            ext,
            tab.cursor_row + 1,
            tab.cursor_col + 1,
            ext,
            content,
            truncated
        )
    }

    pub fn ai_insert_code(&mut self) {
        if let Some(code) = self.ai_state.extract_last_code_block() {
            // Switch focus to editor and paste the code
            self.focus = Focus::Editor;
            for ch in code.chars() {
                if ch == '\n' {
                    self.enter();
                } else if ch != '\r' {
                    self.insert_char(ch);
                }
            }
            self.status_msg = "AI: Code inserted into editor".into();
        } else {
            self.status_msg = "AI: No code block found in response".into();
        }
    }

    pub fn ai_input_char(&mut self, ch: char) {
        self.ai_state
            .input_buffer
            .insert(self.ai_state.input_cursor, ch);
        self.ai_state.input_cursor += ch.len_utf8();
    }

    pub fn ai_input_backspace(&mut self) {
        if self.ai_state.input_cursor > 0 {
            let prev = self.ai_state.input_buffer[..self.ai_state.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.ai_state.input_buffer.remove(prev);
            self.ai_state.input_cursor = prev;
        }
    }

    pub fn ai_input_delete(&mut self) {
        if self.ai_state.input_cursor < self.ai_state.input_buffer.len() {
            self.ai_state
                .input_buffer
                .remove(self.ai_state.input_cursor);
        }
    }

    pub fn ai_input_left(&mut self) {
        if self.ai_state.input_cursor > 0 {
            let prev = self.ai_state.input_buffer[..self.ai_state.input_cursor]
                .char_indices()
                .last()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.ai_state.input_cursor = prev;
        }
    }

    pub fn ai_input_right(&mut self) {
        if self.ai_state.input_cursor < self.ai_state.input_buffer.len() {
            let ch = self.ai_state.input_buffer[self.ai_state.input_cursor..]
                .chars()
                .next()
                .unwrap();
            self.ai_state.input_cursor += ch.len_utf8();
        }
    }

    pub fn ai_scroll_up(&mut self) {
        if self.ai_state.scroll_offset > 0 {
            self.ai_state.scroll_offset -= 1;
        }
    }

    pub fn ai_scroll_down(&mut self) {
        self.ai_state.scroll_offset += 1;
    }

    pub fn ai_clear_history(&mut self) {
        self.ai_state.display_messages.clear();
        self.ai_state.api_messages.clear();
        self.ai_state.streaming_buffer.clear();
        self.ai_state.scroll_offset = 0;
        self.status_msg = "AI: Chat history cleared".into();
    }
}

pub fn read_directory(path: &Path) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    if let Some(parent) = path.parent() {
        entries.push(FileEntry {
            name: "..".into(),
            path: parent.to_path_buf(),
            is_dir: true,
        });
    }
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    if let Ok(rd) = fs::read_dir(path) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let p = e.path();
            let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let entry = FileEntry {
                name,
                path: p,
                is_dir,
            };
            if is_dir {
                dirs.push(entry);
            } else {
                files.push(entry);
            }
        }
    }
    dirs.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    files.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    entries.extend(dirs);
    entries.extend(files);
    entries
}

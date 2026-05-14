pub mod ai;
mod app;
pub mod config;
mod events;
pub mod fuzzy;
pub mod lsp;
mod plugin;
pub mod term;
mod ui;

use std::env;
use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use app::App;
use events::UiRects;

#[tokio::main]
async fn main() -> io::Result<()> {
    let cwd = env::current_dir().unwrap_or_else(|_| ".".into());

    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new(cwd.clone());
    let mut rects = UiRects::default();
    let mut highlighter = ui::Highlighter::new();

    // Populate available themes from syntect
    app.theme_list = highlighter.theme_names();

    // ── Plugin system ──────────────────────────────────────────────
    let mut plugin_engine = plugin::PluginEngine::new();
    plugin_engine.load_plugins(&cwd.join("plugins"));
    let plugin_count = plugin_engine.loaded_count();
    if plugin_count > 0 {
        app.status_msg = format!("{} plugin(s) loaded", plugin_count);
    }
    // Fire on_init hooks.
    plugin_engine.run_on_init(&mut app);

    // ── Main loop ──────────────────────────────────────────────────
    // First frame: draw before any events so the UI appears immediately.
    terminal.draw(|f| {
        rects = ui::draw(f, &app, &mut highlighter);
    })?;

    // ── PTY Setup ───────────────────────────────────────────────────
    let (pty_event_tx, pty_event_rx) = std::sync::mpsc::channel();
    let mut pty = term::pty::spawn_pty(pty_event_tx);
    let (pty_tx, pty_rx) = std::sync::mpsc::channel::<term::pty::PtyCommand>();
    std::thread::spawn(move || {
        while let Ok(cmd) = pty_rx.recv() {
            match cmd {
                term::pty::PtyCommand::Data(data) => pty.write(&data),
                term::pty::PtyCommand::Resize(rows, cols) => pty.resize(rows, cols),
            }
        }
    });
    app.pty_tx = Some(pty_tx);

    // ── LSP Setup ───────────────────────────────────────────────────
    let (mut lsp_client, mut lsp_rx) = lsp::LspClient::new();
    if let Ok(root_url) = url::Url::from_file_path(&cwd) {
        lsp_client.initialize(root_url);
    }
    app.lsp_client = Some(lsp_client);
    let mut last_autosave = std::time::Instant::now();
    let mut last_git_update = std::time::Instant::now();
    let mut last_term_size = (0u16, 0u16);

    loop {
        let autosave_interval = std::time::Duration::from_millis(app.config.autosave_interval_ms);
        if last_autosave.elapsed() >= autosave_interval {
            app.save_all();
            last_autosave = std::time::Instant::now();
        }
        if last_git_update.elapsed() >= std::time::Duration::from_millis(500) {
            if !app.tabs.is_empty() {
                let active = app.active_tab;
                let cwd = app.cwd.clone();
                let repo = app.git_repo.as_ref();
                app.tabs[active].update_git_marks(repo, &cwd);
            }
            last_git_update = std::time::Instant::now();
        }
        // Handle background events
        while let Ok(data) = pty_event_rx.try_recv() {
            app.terminal_parser.advance(&mut app.terminal, &data);
        }

        while let Ok(msg) = lsp_rx.try_recv() {
            // Simplified handling for MVP
            match msg {
                lsp::LspMessage::Notification(method, params) => {
                    if method == "textDocument/publishDiagnostics" {
                        if let Ok(params) =
                            serde_json::from_value::<lsp_types::PublishDiagnosticsParams>(params)
                        {
                            app.lsp_diagnostics = params.diagnostics;
                        }
                    }
                }
                lsp::LspMessage::Response(id, result) => {
                    if id == 1 {
                        // Initialize response received!
                        app.lsp_initialized = true;
                        if let Some(client) = &app.lsp_client {
                            client.initialized();
                            for tab in &app.tabs {
                                if let Some(path) = &tab.file_path {
                                    let abs_path = if path.is_absolute() {
                                        path.clone()
                                    } else {
                                        cwd.join(path)
                                    };
                                    if let Ok(url) = url::Url::from_file_path(abs_path) {
                                        client.did_open(
                                            url,
                                            tab.lines.join("\n"),
                                            tab.text_version as i32,
                                        );
                                    }
                                }
                            }
                        }
                    } else if let Ok(completions) =
                        serde_json::from_value::<lsp_types::CompletionResponse>(result)
                    {
                        match completions {
                            lsp_types::CompletionResponse::Array(arr) => {
                                app.lsp_completions = Some(arr)
                            }
                            lsp_types::CompletionResponse::List(list) => {
                                app.lsp_completions = Some(list.items)
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        // Handle AI agent events
        if let Some(rx) = &app.ai_event_rx {
            while let Ok(event) = rx.try_recv() {
                match event {
                    ai::AiEvent::Chunk(text) => {
                        app.ai_state.streaming_buffer.push_str(&text);
                        app.ai_state.stream_tick = app.ai_state.stream_tick.wrapping_add(1);
                    }
                    ai::AiEvent::ToolUse(exec) => {
                        app.ai_state
                            .display_messages
                            .push(ai::DisplayMessage::ToolUse(exec));
                        app.status_msg = "AI Agent: Using tools...".into();
                    }
                    ai::AiEvent::Done => {
                        if !app.ai_state.streaming_buffer.is_empty() {
                            let response = app.ai_state.streaming_buffer.clone();
                            app.ai_state
                                .display_messages
                                .push(ai::DisplayMessage::Assistant(response.clone()));
                            app.ai_state
                                .api_messages
                                .push(ai::ChatMessage::assistant(&response));
                            app.ai_state.streaming_buffer.clear();
                        }
                        app.ai_state.is_streaming = false;
                        app.ai_event_rx = None;
                        app.status_msg = "AI Agent: Done".into();
                        break; // rx is now None
                    }
                    ai::AiEvent::Error(msg) => {
                        app.ai_state
                            .display_messages
                            .push(ai::DisplayMessage::Assistant(format!("⚠ Error: {}", msg)));
                        app.status_msg = format!("AI Error: {}", msg);
                    }
                    ai::AiEvent::FileModified(path) => {
                        // Refresh file tree and reload if open
                        app.file_tree = app::read_directory(&app.cwd);
                        let full_path = if std::path::Path::new(&path).is_absolute() {
                            std::path::PathBuf::from(&path)
                        } else {
                            app.cwd.join(&path)
                        };
                        // Reload the file if it's already open in a tab
                        for tab in &mut app.tabs {
                            if tab.file_path.as_deref() == Some(full_path.as_path()) {
                                if let Ok(content) = std::fs::read_to_string(&full_path) {
                                    tab.lines = content.lines().map(String::from).collect();
                                    if tab.lines.is_empty() {
                                        tab.lines.push(String::new());
                                    }
                                    tab.dirty = false;
                                    tab.text_version += 1;
                                }
                            }
                        }
                        app.status_msg = format!("AI: Modified {}", path);
                    }
                }
            }
        }

        if !events::handle_events(&mut app, rects)? {
            tokio::task::yield_now().await;
        }

        // Fire on_save hooks when a save just completed.
        if app.just_saved {
            plugin_engine.run_on_save(&mut app);
            app.just_saved = false;
        }

        if app.should_quit {
            app.save_session();
            break;
        }

        // Ensure cursor is visible, then redraw.
        let vh = rects.text_area.height as usize;
        app.ensure_cursor_visible(vh);

        terminal.draw(|f| {
            rects = ui::draw(f, &app, &mut highlighter);
        })?;

        let cur_term_size = (rects.terminal.width, rects.terminal.height);
        if app.show_terminal
            && cur_term_size != last_term_size
            && cur_term_size.0 > 0
            && cur_term_size.1 > 0
        {
            if let Some(tx) = &app.pty_tx {
                let _ = tx.send(crate::term::pty::PtyCommand::Resize(
                    cur_term_size.1,
                    cur_term_size.0,
                )); // rows, cols
            }
            last_term_size = cur_term_size;
        }
    }

    // ── Cleanup ────────────────────────────────────────────────────
    terminal::disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

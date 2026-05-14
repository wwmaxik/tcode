/// Plugin system for tcode — loads and executes .rhai scripts from a plugins/ directory.
use rhai::{AST, Dynamic, Engine, Scope};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;

use crate::app::App;

// ── Commands that scripts can produce ───────────────────────────────

#[derive(Clone, Debug)]
enum PluginCommand {
    InsertText(String),
    Log(String),
}

// ── Editor API object exposed to Rhai scripts ───────────────────────

/// This is the object scripts receive as `editor`.
/// Uses `Rc<RefCell<>>` for the command queue so mutations inside
/// Rhai (which clones the value) still write to the shared queue.
#[derive(Clone)]
struct EditorApi {
    current_line: String,
    cursor_row: i64,
    cursor_col: i64,
    tab_name: String,
    cmds: Rc<RefCell<Vec<PluginCommand>>>,
}

// ── Loaded plugin ───────────────────────────────────────────────────

struct LoadedPlugin {
    name: String,
    ast: AST,
    has_on_init: bool,
    has_on_save: bool,
}

// ── Plugin engine ───────────────────────────────────────────────────

pub struct PluginEngine {
    engine: Engine,
    plugins: Vec<LoadedPlugin>,
}

impl PluginEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register the EditorApi custom type and its methods.
        engine.register_type_with_name::<EditorApi>("Editor");

        engine.register_fn("insert_text", |api: &mut EditorApi, text: String| {
            api.cmds.borrow_mut().push(PluginCommand::InsertText(text));
        });

        engine.register_fn("get_current_line", |api: &mut EditorApi| -> String {
            api.current_line.clone()
        });

        engine.register_fn("log", |api: &mut EditorApi, msg: String| {
            api.cmds.borrow_mut().push(PluginCommand::Log(msg));
        });

        engine.register_get("line", |api: &mut EditorApi| api.cursor_row);
        engine.register_get("col", |api: &mut EditorApi| api.cursor_col);
        engine.register_get("file", |api: &mut EditorApi| api.tab_name.clone());

        Self {
            engine,
            plugins: Vec::new(),
        }
    }

    /// Scan `dir` for `.rhai` files and compile them.
    pub fn load_plugins(&mut self, dir: &Path) {
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rhai") {
                continue;
            }
            let Ok(source) = fs::read_to_string(&path) else {
                continue;
            };
            match self.engine.compile(&source) {
                Ok(ast) => {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let has_on_init = source.contains("fn on_init(");
                    let has_on_save = source.contains("fn on_save(");
                    self.plugins.push(LoadedPlugin {
                        name,
                        ast,
                        has_on_init,
                        has_on_save,
                    });
                }
                Err(e) => {
                    eprintln!("Plugin compile error {}: {}", path.display(), e);
                }
            }
        }
    }

    pub fn loaded_count(&self) -> usize {
        self.plugins.len()
    }

    /// Build an EditorApi snapshot from current App state.
    fn make_api(app: &App) -> (EditorApi, Rc<RefCell<Vec<PluginCommand>>>) {
        let cmds = Rc::new(RefCell::new(Vec::new()));
        let tab = &app.tabs[app.active_tab];
        let api = EditorApi {
            current_line: tab.lines.get(tab.cursor_row).cloned().unwrap_or_default(),
            cursor_row: (tab.cursor_row + 1) as i64,
            cursor_col: (tab.cursor_col + 1) as i64,
            tab_name: tab.name.clone(),
            cmds: cmds.clone(),
        };
        (api, cmds)
    }

    /// Apply collected commands back to the App.
    fn apply_commands(app: &mut App, cmds: &[PluginCommand]) {
        for cmd in cmds {
            match cmd {
                PluginCommand::InsertText(text) => {
                    for ch in text.chars() {
                        app.insert_char(ch);
                    }
                }
                PluginCommand::Log(msg) => {
                    app.status_msg.clone_from(msg);
                }
            }
        }
    }

    /// Call a named hook function across all plugins that define it.
    fn run_hook(&self, app: &mut App, hook_name: &str, check: fn(&LoadedPlugin) -> bool) {
        for plugin in &self.plugins {
            if !check(plugin) {
                continue;
            }
            let (api, cmds) = Self::make_api(app);
            let mut scope = Scope::new();
            match self
                .engine
                .call_fn::<Dynamic>(&mut scope, &plugin.ast, hook_name, (api,))
            {
                Ok(_) => {}
                Err(e) => {
                    app.status_msg = format!("Plugin '{}': {}", plugin.name, e);
                    continue;
                }
            }
            Self::apply_commands(app, &cmds.borrow());
        }
    }

    /// Run `on_init(editor)` in every plugin that defines it.
    pub fn run_on_init(&self, app: &mut App) {
        self.run_hook(app, "on_init", |p| p.has_on_init);
    }

    /// Run `on_save(editor)` in every plugin that defines it.
    pub fn run_on_save(&self, app: &mut App) {
        self.run_hook(app, "on_save", |p| p.has_on_save);
    }
}

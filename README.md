# tcode v0.0.1-beta

**tcode** is a professional-grade Terminal User Interface (TUI) code editor written in Rust. It combines the speed and efficiency of terminal-based editing with the modern features of a full-blown IDE, such as LSP integration, an integrated terminal, fuzzy finding, and a robust plugin system.

![License](https://img.shields.io/badge/license-GPL--3.0-blue.svg)
![Rust](https://img.shields.io/badge/rust-v1.75+-orange.svg)

## Features

- **AI Agent (Agentic Chat)**: 
  - Integrated AI assistant in the right sidebar (`Ctrl+E`).
  - **Tool Use**: Autonomous file reading, writing, terminal command execution, and directory exploration.
  - **Context-Aware**: Understands your current file, selection, and project structure.
  - **Code Insertion**: Insert suggested code blocks directly into the editor (`Ctrl+L`).
- **File Explorer**: Full-featured sidebar with mouse support for navigating project structures.
- **Syntax Highlighting**: High-performance highlighting powered by `tree-sitter`.
- **Integrated Terminal**: Full ANSI escape sequence support, scrollback buffer, and interactive shell integration.
- **LSP Intelligence**: Professional IDE features via `rust-analyzer` (and other servers):
  - Real-time autocompletion menu.
  - Inline diagnostic markers (Errors/Warnings) in the gutter.
- **Fuzzy Finder**: Blazing fast file searching using `nucleo` (`Ctrl+P`).
- **Modern Selection**: 
  - Mouse drag selection.
  - `Shift + Arrows` selection.
  - `Ctrl + A` select all.
- **System Clipboard**: Seamless `Ctrl+C` and `Ctrl+V` integration with your system clipboard.
- **Plugin System**: Extend the editor using **Rhai** scripting.
- **Theme System**: Beautiful built-in themes with a live-switching modal.
- **Configurable**: TOML-based configuration located at `~/.config/tcode/config.toml`.

## Keyboard Shortcuts

### General
| Key | Action |
|-----|--------|
| `Ctrl + H` | Toggle help overlay |
| `Ctrl + Q` | Quit editor |
| `Ctrl + S` | Save current file |
| `Ctrl + O` | Open directory prompt |
| `Ctrl + B` | Toggle file explorer |
| `Ctrl + P` | Fuzzy finder (Files) |
| `Ctrl + W` | Close tab |
| `Ctrl + N` | New / Next tab |
| `Ctrl + \`` / `Ctrl + J` | Toggle terminal |
| `Ctrl + T` | Theme switcher |
| `Ctrl + U` | Plugin manager |
| `Ctrl + L` | Open settings |
| `Ctrl + E` | **Toggle AI Agent Sidebar** |

### AI Agent (When focused)
| Key | Action |
|-----|--------|
| `Enter` | Send message to AI |
| `Ctrl + L` | Insert last code block into editor |
| `Ctrl + D` | Clear chat history |
| `Esc` | Return focus to editor |

### Terminal & IDE
| Key | Action |
|-----|--------|
| `Shift + Up/Down` | Scroll terminal history |
| `Up/Down/Enter` | Navigate & apply autocomplete |

### Editor & Selection
| Key | Action |
|-----|--------|
| `Shift + Arrows` | Select text |
| `Ctrl + C` | Copy selection (or line) |
| `Ctrl + V` | Paste from clipboard |
| `Ctrl + A` | Select all |
| `Esc` | Focus file explorer |
| `Alt + ←/→` | Switch between tabs |

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) (v1.75 or later)
- `rust-analyzer` (for Rust LSP support)

### Build from Source
```bash
git clone https://github.com/yourusername/tcode.git
cd tcode
cargo build --release
```

The binary will be available at `./target/release/tcode`.

## Plugins

Plugins are stored in `~/.config/tcode/plugins/` (or the `plugins/` directory in the project root). They use the **Rhai** scripting language.

Example `hello.rhai`:
```rust
fn on_load() {
    print("Hello from Rhai!");
}

fn on_save(file_path) {
    print("Saved: " + file_path);
}
```

## Configuration

The configuration file is located at `~/.config/tcode/config.toml`. See `config.toml.example` for all options.

### AI Setup
1. Open `~/.config/tcode/config.toml` (create it if it doesn't exist).
2. Add your API key and provider info:
```toml
[ai]
api_key = "sk-..."
model = "gpt-4o-mini"
```

## Themes

tcode comes with several curated themes (Mocha, Oceanic, Deepblue, etc.). You can switch them instantly using `Ctrl + T`.

## Roadmap

- [ ] **Multi-cursor support**: Edit multiple lines simultaneously.
- [ ] **Extended LSP support**: Auto-detection and setup for more languages (Go, Python, C++, etc.).
- [ ] **AI Context++**: Automatically feed open tab contents and diagnostics into the AI agent's context.
- [ ] **Plugin API Expansion**: More hooks for UI customization via Rhai.
- [ ] **Vim Mode**: Optional modal editing support.

## Contributing

Contributions are welcome! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to get started.

## License

This project is licensed under the GNU General Public License v3.0 - see the [LICENSE](LICENSE) file for details.
# Changelog

All notable changes to this project will be documented in this file.

## [0.0.1-beta] - 2026-05-14

### Added
- **AI Agentic Chat**: A new proactive AI assistant integrated into the right sidebar.
  - Can read and write files autonomously.
  - Can run terminal commands and list directories.
  - Supports OpenAI-compatible APIs (OpenAI, Groq, OpenRouter, Ollama).
- **AI Panel UI**: A dedicated sidebar with streaming support, markdown highlighting, and tool execution status.
- **One-click code insertion**: `Ctrl+L` to insert AI-generated code directly into the active editor buffer.
- **New Hotkeys**:
  - `Ctrl+E`: Toggle AI Agent sidebar.
  - `Ctrl+D`: Clear AI chat history.
- **GitHub Actions**: Automated CI workflow for Rust builds.
- **Improved .gitignore**: Better protection for local configurations and secrets.

### Changed
- Refactored core app state to support asynchronous AI events.
- Updated `README.md` with comprehensive usage instructions.
- Set project version to `0.0.1-beta`.

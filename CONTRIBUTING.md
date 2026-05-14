# Contributing to tcode

First off, thank you for considering contributing to **tcode**! It's people like you that make tcode a great tool for everyone.

## How Can I Contribute?

### Reporting Bugs
- Check the [Issues](https://github.com/yourusername/tcode/issues) to see if the bug has already been reported.
- If not, create a new issue. Include as much detail as possible: your OS, terminal emulator, steps to reproduce, and what you expected vs. what happened.

### Suggesting Enhancements
- Open an issue with the "enhancement" label.
- Explain why this feature would be useful and how you imagine it working.

### Pull Requests
1. Fork the repo and create your branch from `master`.
2. If you've added code that should be tested, add tests.
3. Ensure the test suite passes (`cargo test`).
4. Make sure your code is formatted correctly (`cargo fmt`).
5. Run Clippy to check for common issues (`cargo clippy`).
6. Update the documentation (like `README.md` or `CHANGELOG.md`) if necessary.
7. Issue that Pull Request!

## Development Setup

1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Clone the repo: `git clone https://github.com/yourusername/tcode.git`
3. Build the project: `cargo build`
4. Run in dev mode: `cargo run -- some_file.rs`

## Code of Conduct
Please be respectful and professional in all interactions. We aim to build a welcoming community for everyone.

## AI Agent Notice
If you are modifying the AI Agent (`src/ai/`), please ensure that tool definitions remain compatible with standard OpenAI function calling protocols.

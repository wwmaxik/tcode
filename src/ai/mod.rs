/// AI Agent module for tcode — Agentic AI with tool use.
///
/// The agent can autonomously:
/// - Read files from the project
/// - Write/modify files
/// - Execute terminal commands
/// - List directory contents
///
/// Uses OpenAI-compatible function calling API.
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

// ── Configuration ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub system_prompt: String,
    pub max_tokens: u32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o-mini".to_string(),
            system_prompt: "You are a powerful coding agent integrated into a TUI code editor called tcode. \
                You have access to tools that let you read files, write files, run terminal commands, and list directories. \
                Use these tools proactively to help the user. When you need to understand code, read the relevant files first. \
                When asked to make changes, write the files directly. \
                Be concise in your text responses. When providing code in text, use fenced code blocks.".to_string(),
            max_tokens: 4096,
        }
    }
}

impl AiConfig {
    pub fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

// ── Chat Messages ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

impl ChatMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn user(content: &str) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }
    pub fn tool_result(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

// ── Tool Definitions ───────────────────────────────────────────────

fn get_tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read the contents of a file. Returns the file content as text. Use this to understand code before making changes.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to read (relative to project root or absolute)"
                        }
                    },
                    "required": ["path"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Use this to create or modify source code files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Path to the file to write (relative to project root or absolute)"
                        },
                        "content": {
                            "type": "string",
                            "description": "The complete file content to write"
                        }
                    },
                    "required": ["path", "content"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Execute a shell command and return its output. Use this for building, testing, running scripts, git operations, etc. Commands run in the project root directory.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "The shell command to execute (e.g. 'cargo build', 'ls -la', 'git status')"
                        }
                    },
                    "required": ["command"]
                }
            }
        },
        {
            "type": "function",
            "function": {
                "name": "list_directory",
                "description": "List files and directories in a given path. Returns names with [DIR] or [FILE] markers.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "Directory path to list (relative to project root or absolute). Use '.' for project root."
                        }
                    },
                    "required": ["path"]
                }
            }
        }
    ])
}

// ── Tool Execution ─────────────────────────────────────────────────

/// Result of a tool execution, shown in the UI.
#[derive(Debug, Clone)]
pub struct ToolExecution {
    pub tool_name: String,
    pub arguments_summary: String,
    pub result_summary: String,
    pub success: bool,
}

async fn execute_tool(
    name: &str,
    arguments: &str,
    cwd: &std::path::Path,
) -> (String, ToolExecution) {
    let args: serde_json::Value =
        serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null);

    match name {
        "read_file" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let full_path = if std::path::Path::new(path_str).is_absolute() {
                std::path::PathBuf::from(path_str)
            } else {
                cwd.join(path_str)
            };

            match tokio::fs::read_to_string(&full_path).await {
                Ok(content) => {
                    let lines = content.lines().count();
                    let truncated = truncate_str(&content, 15000);
                    let exec = ToolExecution {
                        tool_name: "read_file".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✓ Read {} lines", lines),
                        success: true,
                    };
                    (truncated, exec)
                }
                Err(e) => {
                    let exec = ToolExecution {
                        tool_name: "read_file".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✗ {}", e),
                        success: false,
                    };
                    (format!("Error: {}", e), exec)
                }
            }
        }
        "write_file" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or("");
            let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
            let full_path = if std::path::Path::new(path_str).is_absolute() {
                std::path::PathBuf::from(path_str)
            } else {
                cwd.join(path_str)
            };

            // Create parent directories if needed
            if let Some(parent) = full_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }

            match tokio::fs::write(&full_path, content).await {
                Ok(()) => {
                    let lines = content.lines().count();
                    let exec = ToolExecution {
                        tool_name: "write_file".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✓ Wrote {} lines", lines),
                        success: true,
                    };
                    (
                        format!("Successfully wrote {} lines to {}", lines, path_str),
                        exec,
                    )
                }
                Err(e) => {
                    let exec = ToolExecution {
                        tool_name: "write_file".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✗ {}", e),
                        success: false,
                    };
                    (format!("Error: {}", e), exec)
                }
            }
        }
        "run_command" => {
            let cmd = args.get("command").and_then(|v| v.as_str()).unwrap_or("");

            let result = tokio::process::Command::new("bash")
                .arg("-c")
                .arg(cmd)
                .current_dir(cwd)
                .output()
                .await;

            match result {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    let exit_code = output.status.code().unwrap_or(-1);
                    let combined = if stderr.is_empty() {
                        truncate_str(&stdout, 10000)
                    } else {
                        truncate_str(&format!("{}\nSTDERR:\n{}", stdout, stderr), 10000)
                    };
                    let success = output.status.success();
                    let exec = ToolExecution {
                        tool_name: "run_command".into(),
                        arguments_summary: truncate_str(cmd, 60),
                        result_summary: format!(
                            "{} exit code {}",
                            if success { "✓" } else { "✗" },
                            exit_code
                        ),
                        success,
                    };
                    (combined, exec)
                }
                Err(e) => {
                    let exec = ToolExecution {
                        tool_name: "run_command".into(),
                        arguments_summary: truncate_str(cmd, 60),
                        result_summary: format!("✗ {}", e),
                        success: false,
                    };
                    (format!("Error: {}", e), exec)
                }
            }
        }
        "list_directory" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let full_path = if std::path::Path::new(path_str).is_absolute() {
                std::path::PathBuf::from(path_str)
            } else {
                cwd.join(path_str)
            };

            match tokio::fs::read_dir(&full_path).await {
                Ok(mut entries) => {
                    let mut items = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false);
                        let marker = if is_dir { "[DIR] " } else { "[FILE]" };
                        items.push(format!("{} {}", marker, name));
                    }
                    items.sort();
                    let result = items.join("\n");
                    let exec = ToolExecution {
                        tool_name: "list_directory".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✓ {} entries", items.len()),
                        success: true,
                    };
                    (result, exec)
                }
                Err(e) => {
                    let exec = ToolExecution {
                        tool_name: "list_directory".into(),
                        arguments_summary: path_str.to_string(),
                        result_summary: format!("✗ {}", e),
                        success: false,
                    };
                    (format!("Error: {}", e), exec)
                }
            }
        }
        _ => {
            let exec = ToolExecution {
                tool_name: name.to_string(),
                arguments_summary: String::new(),
                result_summary: "✗ Unknown tool".into(),
                success: false,
            };
            (format!("Unknown tool: {}", name), exec)
        }
    }
}

// ── AI Events (sent to main UI thread) ─────────────────────────────

#[derive(Debug)]
pub enum AiEvent {
    /// A delta text chunk from the streaming response.
    Chunk(String),
    /// A tool is being called (for UI display).
    ToolUse(ToolExecution),
    /// Stream / agent loop finished.
    Done,
    /// An error occurred.
    Error(String),
    /// A file was modified by the agent — UI should refresh.
    FileModified(String),
}

// ── AI State ───────────────────────────────────────────────────────

/// Display-friendly message for the UI (not the raw API messages).
#[derive(Debug, Clone)]
pub enum DisplayMessage {
    User(String),
    Assistant(String),
    ToolUse(ToolExecution),
}

pub struct AiState {
    /// Display messages for the chat UI.
    pub display_messages: Vec<DisplayMessage>,
    /// Full API message history (includes tool calls/results).
    pub api_messages: Vec<ChatMessage>,
    pub input_buffer: String,
    pub input_cursor: usize,
    pub scroll_offset: usize,
    pub is_streaming: bool,
    /// Accumulates the current assistant response while streaming.
    pub streaming_buffer: String,
    /// Animation tick for the streaming indicator.
    pub stream_tick: u8,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            display_messages: Vec::new(),
            api_messages: Vec::new(),
            input_buffer: String::new(),
            input_cursor: 0,
            scroll_offset: 0,
            is_streaming: false,
            streaming_buffer: String::new(),
            stream_tick: 0,
        }
    }
}

impl AiState {
    /// Extract code blocks from the last assistant message.
    pub fn extract_last_code_block(&self) -> Option<String> {
        for msg in self.display_messages.iter().rev() {
            if let DisplayMessage::Assistant(content) = msg {
                return extract_code_from_markdown(content);
            }
        }
        if !self.streaming_buffer.is_empty() {
            return extract_code_from_markdown(&self.streaming_buffer);
        }
        None
    }
}

/// Extract the last fenced code block from markdown text.
fn extract_code_from_markdown(text: &str) -> Option<String> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_block = String::new();

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if in_block {
                blocks.push(current_block.clone());
                current_block.clear();
                in_block = false;
            } else {
                in_block = true;
                current_block.clear();
            }
        } else if in_block {
            if !current_block.is_empty() {
                current_block.push('\n');
            }
            current_block.push_str(line);
        }
    }

    blocks.last().cloned()
}

// ── Agent Loop ─────────────────────────────────────────────────────

/// Run the full agentic loop: send message → handle tool calls → repeat → stream final response.
/// This runs in a background tokio task.
pub fn run_agent(
    config: &AiConfig,
    messages: Vec<ChatMessage>,
    cwd: std::path::PathBuf,
    tx: mpsc::Sender<AiEvent>,
) {
    let url = format!("{}/chat/completions", config.base_url.trim_end_matches('/'));
    let api_key = config.api_key.clone();
    let model = config.model.clone();
    let max_tokens = config.max_tokens;
    let tools = get_tool_definitions();

    tokio::spawn(async move {
        let client = match reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(180))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(AiEvent::Error(format!("HTTP client error: {}", e)));
                let _ = tx.send(AiEvent::Done);
                return;
            }
        };

        let mut current_messages = messages;
        let max_iterations = 15; // Safety limit for agent loops

        for _iteration in 0..max_iterations {
            // Decide: if this might be the final response, use streaming.
            // For tool-call detection, we first try non-streaming.
            let body = serde_json::json!({
                "model": model,
                "messages": current_messages,
                "max_tokens": max_tokens,
                "tools": tools,
                "stream": false,
            });

            let response = client
                .post(&url)
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .send()
                .await;

            let response = match response {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx.send(AiEvent::Error(format!("Request failed: {}", e)));
                    let _ = tx.send(AiEvent::Done);
                    return;
                }
            };

            if !response.status().is_success() {
                let status = response.status();
                let body_text = response.text().await.unwrap_or_default();
                let _ = tx.send(AiEvent::Error(format!(
                    "API error {}: {}",
                    status,
                    truncate_str(&body_text, 300)
                )));
                let _ = tx.send(AiEvent::Done);
                return;
            }

            let response_body: serde_json::Value = match response.json().await {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.send(AiEvent::Error(format!("JSON parse error: {}", e)));
                    let _ = tx.send(AiEvent::Done);
                    return;
                }
            };

            let choice = match response_body.get("choices").and_then(|c| c.get(0)) {
                Some(c) => c,
                None => {
                    let _ = tx.send(AiEvent::Error("No choices in response".into()));
                    let _ = tx.send(AiEvent::Done);
                    return;
                }
            };

            let finish_reason = choice
                .get("finish_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let message = match choice.get("message") {
                Some(m) => m,
                None => {
                    let _ = tx.send(AiEvent::Error("No message in choice".into()));
                    let _ = tx.send(AiEvent::Done);
                    return;
                }
            };

            // Check if the model wants to call tools
            if finish_reason == "tool_calls" || message.get("tool_calls").is_some() {
                if let Some(tool_calls) = message.get("tool_calls").and_then(|v| v.as_array()) {
                    // Add the assistant message with tool_calls to history
                    let tc_parsed: Vec<ToolCall> = tool_calls
                        .iter()
                        .filter_map(|tc| {
                            Some(ToolCall {
                                id: tc.get("id")?.as_str()?.to_string(),
                                call_type: tc
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("function")
                                    .to_string(),
                                function: FunctionCall {
                                    name: tc.get("function")?.get("name")?.as_str()?.to_string(),
                                    arguments: tc
                                        .get("function")?
                                        .get("arguments")?
                                        .as_str()?
                                        .to_string(),
                                },
                            })
                        })
                        .collect();

                    current_messages.push(ChatMessage::assistant_tool_calls(tc_parsed.clone()));

                    // Execute each tool call
                    for tc in &tc_parsed {
                        let (result, exec) =
                            execute_tool(&tc.function.name, &tc.function.arguments, &cwd).await;

                        // Notify UI about the tool execution
                        let _ = tx.send(AiEvent::ToolUse(exec));

                        // If file was modified, notify UI to refresh
                        if tc.function.name == "write_file" {
                            if let Ok(args) =
                                serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
                            {
                                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                                    let _ = tx.send(AiEvent::FileModified(path.to_string()));
                                }
                            }
                        }

                        // Add tool result to messages
                        current_messages.push(ChatMessage::tool_result(&tc.id, &result));
                    }

                    // Continue the loop — the model will process tool results
                    continue;
                }
            }

            // No tool calls — this is the final text response
            if let Some(content) = message.get("content").and_then(|v| v.as_str()) {
                // Send the response as chunks for a streaming-like effect
                let chunk_size = 3; // characters per chunk for typing effect
                let chars: Vec<char> = content.chars().collect();
                for chunk in chars.chunks(chunk_size) {
                    let text: String = chunk.iter().collect();
                    let _ = tx.send(AiEvent::Chunk(text));
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            }

            let _ = tx.send(AiEvent::Done);
            return;
        }

        // If we hit the iteration limit
        let _ = tx.send(AiEvent::Error(
            "Agent reached max iteration limit (15)".into(),
        ));
        let _ = tx.send(AiEvent::Done);
    });
}

fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

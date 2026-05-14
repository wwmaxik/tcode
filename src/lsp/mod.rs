use lsp_types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::process::Stdio;
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
// Removed unused ChildStdin
use tokio::sync::mpsc::{self, Receiver, Sender};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug)]
pub enum LspMessage {
    Notification(String, Value),
    Response(i64, Value),
    Error(i64, Value),
}

pub struct LspClient {
    req_tx: Sender<String>,
    next_id: i64,
}

impl LspClient {
    pub fn new() -> (Self, Receiver<LspMessage>) {
        let mut child = tokio::process::Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start rust-analyzer. Ensure it is in your PATH.");

        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = BufReader::new(child.stdout.take().unwrap());

        let (msg_tx, msg_rx) = mpsc::channel(100);
        let (req_tx, mut req_rx) = mpsc::channel::<String>(100);

        // Reader task
        tokio::spawn(async move {
            loop {
                let mut header = String::new();
                if stdout.read_line(&mut header).await.unwrap_or(0usize) == 0 {
                    break;
                }

                let mut content_length = 0;
                if let Some(stripped) = header.strip_prefix("Content-Length: ") {
                    content_length = stripped.trim().parse().unwrap_or(0);
                }

                // Read headers until empty line
                loop {
                    let mut line = String::new();
                    let _ = stdout.read_line(&mut line).await.unwrap_or(0usize);
                    if line.trim().is_empty() {
                        break;
                    }
                    if let Some(stripped) = line.strip_prefix("Content-Length: ") {
                        content_length = stripped.trim().parse().unwrap_or(0);
                    }
                }

                if content_length > 0 {
                    let mut buf = vec![0; content_length];
                    if stdout.read_exact(&mut buf).await.is_err() {
                        break;
                    }
                    if let Ok(payload) = serde_json::from_slice::<Value>(&buf) {
                        if payload.get("id").is_some() && payload.get("method").is_some() {
                            // Request from server, typically ignore for MVP
                        } else if payload.get("id").is_some() {
                            let id = payload["id"].as_i64().unwrap_or(0);
                            if let Some(err) = payload.get("error") {
                                let _ = msg_tx.send(LspMessage::Error(id, err.clone())).await;
                            } else if let Some(res) = payload.get("result") {
                                let _ = msg_tx.send(LspMessage::Response(id, res.clone())).await;
                            }
                        } else if payload.get("method").is_some() {
                            let method = payload["method"].as_str().unwrap().to_string();
                            let params = payload.get("params").cloned().unwrap_or(Value::Null);
                            let _ = msg_tx.send(LspMessage::Notification(method, params)).await;
                        }
                    }
                }
            }
        });

        // Writer task
        tokio::spawn(async move {
            while let Some(msg) = req_rx.recv().await {
                let formatted = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);
                if stdin.write_all(formatted.as_bytes()).await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        (Self { req_tx, next_id: 1 }, msg_rx)
    }

    pub fn send_request(&mut self, method: &str, params: Value) -> i64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };
        let msg = serde_json::to_string(&req).unwrap();
        let tx = self.req_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(msg).await;
        });
        id
    }

    pub fn send_notification(&self, method: &str, params: Value) {
        let notif = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
        };
        let msg = serde_json::to_string(&notif).unwrap();
        let tx = self.req_tx.clone();
        tokio::spawn(async move {
            let _ = tx.send(msg).await;
        });
    }

    pub fn initialize(&mut self, root_url: url::Url) -> i64 {
        let root_uri = lsp_types::Uri::from_str(root_url.as_ref()).unwrap();
        #[allow(deprecated)]
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(root_uri.clone()),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: root_uri,
                name: "root".to_string(),
            }]),
            capabilities: ClientCapabilities::default(),
            ..Default::default()
        };
        self.send_request("initialize", serde_json::to_value(params).unwrap())
    }

    pub fn initialized(&self) {
        self.send_notification("initialized", serde_json::json!({}));
    }

    pub fn did_open(&self, url: url::Url, text: String, version: i32) {
        let uri = lsp_types::Uri::from_str(url.as_ref()).unwrap();
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "rust".to_string(), // MVP hardcoded
                version,
                text,
            },
        };
        self.send_notification(
            "textDocument/didOpen",
            serde_json::to_value(params).unwrap(),
        );
    }

    pub fn did_change(&self, url: url::Url, text: String, version: i32) {
        let uri = lsp_types::Uri::from_str(url.as_ref()).unwrap();
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        };
        self.send_notification(
            "textDocument/didChange",
            serde_json::to_value(params).unwrap(),
        );
    }

    pub fn completion(&mut self, url: url::Url, line: u32, character: u32) -> i64 {
        let uri = lsp_types::Uri::from_str(url.as_ref()).unwrap();
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        self.send_request(
            "textDocument/completion",
            serde_json::to_value(params).unwrap(),
        )
    }

    pub fn definition(&mut self, uri: lsp_types::Uri, line: u32, character: u32) -> i64 {
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.send_request(
            "textDocument/definition",
            serde_json::to_value(params).unwrap(),
        )
    }
}

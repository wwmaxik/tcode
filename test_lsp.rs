use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use std::process::Stdio;

#[tokio::main]
async fn main() {
    let mut child = tokio::process::Command::new("rust-analyzer")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    // Initialize
    let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#;
    let formatted = format!("Content-Length: {}\r\n\r\n{}", msg.len(), msg);
    stdin.write_all(formatted.as_bytes()).await.unwrap();
    stdin.flush().await.unwrap();

    // Read response
    let mut header = String::new();
    stdout.read_line(&mut header).await.unwrap();
    println!("Header: {:?}", header);
}
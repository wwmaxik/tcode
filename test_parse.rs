use lsp_types::{InitializeParams, ClientCapabilities};
fn main() {
    let s = "file:///tmp";
    let uri: lsp_types::Url = s.parse().unwrap();
}

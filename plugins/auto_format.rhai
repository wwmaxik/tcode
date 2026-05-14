// Auto-format Rust files on save using rustfmt
fn on_save(file_path) {
    if file_path.ends_with(".rs") {
        print("Auto-formatting " + file_path + "...");
        // This is a placeholder for future shell execution from Rhai
        // Once tcode adds shell_exec to Rhai, you could do:
        // exec("rustfmt " + file_path);
    }
}

// Simple logger and greeting plugin
fn on_load() {
    print("Welcome to tcode v0.0.1-beta!");
    print("Plugin system initialized successfully.");
}

fn on_file_open(file_path) {
    print("Editing: " + file_path);
}

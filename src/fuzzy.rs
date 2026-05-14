use std::path::Path;

pub fn scan_directory(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    let mut dirs_to_scan = vec![dir.to_path_buf()];

    while let Some(current_dir) = dirs_to_scan.pop() {
        if let Ok(entries) = std::fs::read_dir(&current_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let file_name = path.file_name().unwrap_or_default().to_string_lossy();

                if file_name.starts_with('.')
                    || file_name == "target"
                    || file_name == "node_modules"
                {
                    continue;
                }

                if path.is_dir() {
                    dirs_to_scan.push(path);
                } else if path.is_file()
                    && let Ok(rel_path) = path.strip_prefix(dir)
                {
                    files.push(rel_path.to_string_lossy().to_string());
                }
            }
        }
    }
    files
}

pub fn match_files(query: &str, files: Vec<String>) -> Vec<String> {
    if query.is_empty() {
        return files.into_iter().take(50).collect();
    }

    let mut matcher = nucleo::Matcher::new(nucleo::Config::DEFAULT);
    let mut matches = Vec::new();
    let mut scratch = Vec::new();

    for file in files {
        if let Some(score) = nucleo::pattern::Pattern::parse(
            query,
            nucleo::pattern::CaseMatching::Ignore,
            nucleo::pattern::Normalization::Smart,
        )
        .score(
            nucleo::Utf32Str::new(file.as_str(), &mut scratch),
            &mut matcher,
        ) {
            matches.push((score, file));
        }
    }

    matches.sort_by(|a, b| b.0.cmp(&a.0));
    matches.into_iter().take(50).map(|(_, f)| f).collect()
}

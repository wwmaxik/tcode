use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub tab_size: u16,
    pub autosave_interval_ms: u64,
    pub theme: String,
    pub show_explorer: bool,
    #[serde(default)]
    pub ai: crate::ai::AiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tab_size: 4,
            autosave_interval_ms: 30000,
            theme: "base16-ocean.dark".to_string(),
            show_explorer: true,
            ai: crate::ai::AiConfig::default(),
        }
    }
}

impl Config {
    pub fn config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = Path::new(&home).join(".config").join("tcode");
        fs::create_dir_all(&dir).ok();
        dir.join("config.toml")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
        let default_config = Self::default();
        default_config.save();
        default_config
    }

    pub fn save(&self) {
        if let Ok(content) = toml::to_string(self) {
            let _ = fs::write(Self::config_path(), content);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Session {
    pub open_files: Vec<String>,
    pub active_file_index: usize,
}

impl Session {
    pub fn session_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dir = Path::new(&home).join(".config").join("tcode");
        fs::create_dir_all(&dir).ok();
        dir.join("session.json")
    }

    pub fn load() -> Self {
        let path = Self::session_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(session) = serde_json::from_str(&content) {
                return session;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(Self::session_path(), content);
        }
    }
}

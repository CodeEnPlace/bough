use crate::PartialSession;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Toml(toml::de::Error),
    Json(serde_json::Error),
    UnknownExtension(PathBuf),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "failed to read config: {e}"),
            Self::Toml(e) => write!(f, "invalid toml: {e}"),
            Self::Json(e) => write!(f, "invalid json: {e}"),
            Self::UnknownExtension(p) => write!(f, "unknown config extension: {}", p.display()),
        }
    }
}

impl std::error::Error for ConfigError {}

pub const SEARCH_PATHS: &[&str] = &[
    ".config/bough.config.json",
    ".config/bough.config.toml",
    ".config/bough.json",
    ".config/bough.toml",
    ".bough.config.json",
    ".bough.config.toml",
    ".bough.json",
    ".bough.toml",
    "bough.config.json",
    "bough.config.toml",
    "bough.json",
    "bough.toml",
];

pub fn read_config(path: &Path) -> Result<PartialSession, ConfigError> {
    let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => toml::from_str(&content).map_err(ConfigError::Toml),
        Some("json") => serde_json::from_str(&content).map_err(ConfigError::Json),
        _ => Err(ConfigError::UnknownExtension(path.to_owned())),
    }
}

pub fn discover_config(from: &Path) -> Option<(PathBuf, Result<PartialSession, ConfigError>)> {
    let mut dir = from.to_path_buf();
    loop {
        for name in SEARCH_PATHS {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some((candidate.clone(), read_config(&candidate)));
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}

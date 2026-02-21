use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LanguageId {
    #[serde(alias = "js")]
    #[value(alias = "js")]
    Javascript,
    #[serde(alias = "ts")]
    #[value(alias = "ts")]
    Typescript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Vcs {
    Git,
    Jj,
    Mercurial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    Random,
    Alphabetical,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Commands {
    pub install: Option<String>,
    pub build: Option<String>,
    pub test: String,
    pub cleanup: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    pub language: Option<LanguageId>,
    pub vcs: Option<Vcs>,
    pub working_dir: Option<PathBuf>,
    pub parallelism: Option<usize>,
    pub report_dir: Option<PathBuf>,
    pub ordering: Option<Ordering>,
    pub sub_dir: Option<PathBuf>,
    pub files: Option<String>,
    #[serde(default)]
    pub ignore_mutants: Vec<String>,
    #[serde(default)]
    pub timeout: Timeout,
    #[serde(default)]
    pub commands: Commands,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Timeout {
    pub absolute: Option<u64>,
    pub relative: Option<f64>,
}

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

const SEARCH_PATHS: &[&str] = &[
    ".config/pollard.config.json",
    ".config/pollard.config.toml",
    ".config/pollard.json",
    ".config/pollard.toml",
    ".pollard.config.json",
    ".pollard.config.toml",
    ".pollard.json",
    ".pollard.toml",
    "pollard.config.json",
    "pollard.config.toml",
    "pollard.json",
    "pollard.toml",
];

impl Config {
    pub fn read(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => toml::from_str(&content).map_err(ConfigError::Toml),
            Some("json") => serde_json::from_str(&content).map_err(ConfigError::Json),
            _ => Err(ConfigError::UnknownExtension(path.to_owned())),
        }
    }

    pub fn discover(from: &Path) -> Option<(PathBuf, Result<Self, ConfigError>)> {
        let mut dir = from.to_path_buf();
        loop {
            for name in SEARCH_PATHS {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some((candidate.clone(), Self::read(&candidate)));
                }
            }
            if !dir.pop() {
                return None;
            }
        }
    }
}

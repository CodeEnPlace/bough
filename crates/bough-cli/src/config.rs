use bough_core::config::Config;
use std::path::{Path, PathBuf};

use crate::Cli;

#[derive(Debug)]
pub enum Error {
    NoConfigFound,
    Read(PathBuf, std::io::Error),
    Parse(PathBuf, String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoConfigFound => write!(f, "no config file found"),
            Error::Read(path, err) => write!(f, "failed to read {}: {err}", path.display()),
            Error::Parse(path, err) => write!(f, "failed to parse {}: {err}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

const SEARCH_PATHS: &[&str] = &[
    ".config/bough.config.toml",
    ".config/bough.config.yml",
    ".config/bough.config.yaml",
    ".config/bough.config.json",
    ".config/.bough.config.toml",
    ".config/.bough.config.yml",
    ".config/.bough.config.yaml",
    ".config/.bough.config.json",
    "bough.config.toml",
    "bough.config.yml",
    "bough.config.yaml",
    "bough.config.json",
    ".bough.config.toml",
    ".bough.config.yml",
    ".bough.config.yaml",
    ".bough.config.json",
];

enum Format {
    Toml,
    Yaml,
    Json,
}

fn format_for(path: &Path) -> Format {
    match path.extension().and_then(|e| e.to_str()) {
        Some("toml") => Format::Toml,
        Some("yml" | "yaml") => Format::Yaml,
        Some("json") => Format::Json,
        _ => Format::Toml,
    }
}

fn read_config(path: &Path) -> Result<Config, Error> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Read(path.to_owned(), e))?;
    match format_for(path) {
        Format::Toml => toml::from_str(&content)
            .map_err(|e| Error::Parse(path.to_owned(), e.to_string())),
        Format::Yaml => serde_yaml::from_str(&content)
            .map_err(|e| Error::Parse(path.to_owned(), e.to_string())),
        Format::Json => serde_json::from_str(&content)
            .map_err(|e| Error::Parse(path.to_owned(), e.to_string())),
    }
}

fn read_as_value(path: &Path) -> Result<serde_value::Value, Error> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| Error::Read(path.to_owned(), e))?;
    let raw: serde_value::Value = match format_for(path) {
        Format::Toml => {
            let tv: toml::Value = toml::from_str(&content)
                .map_err(|e| Error::Parse(path.to_owned(), e.to_string()))?;
            serde_value::to_value(tv)
                .map_err(|e| Error::Parse(path.to_owned(), e.to_string()))?
        }
        Format::Yaml => serde_yaml::from_str(&content)
            .map_err(|e| Error::Parse(path.to_owned(), e.to_string()))?,
        Format::Json => serde_json::from_str(&content)
            .map_err(|e| Error::Parse(path.to_owned(), e.to_string()))?,
    };
    Ok(raw)
}

fn discover_config() -> Option<PathBuf> {
    SEARCH_PATHS.iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

pub fn load(cli: &Cli) -> Result<Config, Error> {
    let config_path = match &cli.config {
        Some(path) => {
            if !path.exists() {
                return Err(Error::Read(
                    path.clone(),
                    std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"),
                ));
            }
            path.clone()
        }
        None => discover_config().ok_or(Error::NoConfigFound)?,
    };

    let mut config = read_config(&config_path)?;

    for override_path in &cli.config_overrides {
        let patch = read_as_value(override_path)?;
        config.override_with(patch);
    }

    for toml_str in &cli.config_sets {
        let tv: toml::Value = toml::from_str(toml_str)
            .map_err(|e| Error::Parse("<--set>".into(), e.to_string()))?;
        let patch = serde_value::to_value(tv)
            .map_err(|e| Error::Parse("<--set>".into(), e.to_string()))?;
        config.override_with(patch);
    }

    config.resolve_paths();

    Ok(config)
}

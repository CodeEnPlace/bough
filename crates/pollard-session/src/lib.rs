mod config;

pub use config::{discover_config, read_config, ConfigError, SEARCH_PATHS};

use pollard_core::config::{LanguageId, Ordering, Vcs};
use pollard_session_derive::Settings;
use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, serde::Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Plain,
    Pretty,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

#[derive(Debug, Clone, Serialize, Settings)]
pub struct Commands {
    #[setting(long = "command-install")]
    pub install: Option<String>,
    #[setting(long = "command-build")]
    pub build: Option<String>,
    #[setting(long = "command-test")]
    pub test: String,
    #[setting(long = "command-cleanup")]
    pub cleanup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Settings)]
pub struct Timeout {
    #[setting(long = "timeout-absolute")]
    pub absolute: u64,
    #[setting(long = "timeout-relative")]
    pub relative: f64,
}

#[derive(Debug, Serialize, Settings)]
pub struct Session {
    pub language: LanguageId,
    pub vcs: Vcs,
    pub working_dir: PathBuf,
    pub parallelism: usize,
    pub report_dir: PathBuf,
    pub ordering: Ordering,
    #[setting(default = "PathBuf::from(\".\")")]
    pub sub_dir: PathBuf,
    pub files: String,
    pub ignore_mutants: Vec<String>,
    #[setting(flatten)]
    pub timeout: Timeout,
    #[setting(default = "Style::Plain")]
    pub style: Style,
    #[setting(default = "DiffStyle::Unified")]
    pub diff: DiffStyle,
    #[setting(env = "NO_COLOR", default = "false")]
    pub no_color: bool,
    #[setting(cli_only, default = "false")]
    pub exec: bool,
    #[setting(skip)]
    pub config_path: PathBuf,
    #[setting(flatten)]
    pub commands: Commands,
}

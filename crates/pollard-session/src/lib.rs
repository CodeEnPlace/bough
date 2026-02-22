mod config;

pub use config::{discover_config, read_config, ConfigError, SEARCH_PATHS};

use pollard_core::config::{LanguageId, Ordering, Vcs};
use pollard_session_derive::Settings;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("failed to get current directory")
            .join(path)
    }
}

impl Session {
    pub fn normalize_paths(&mut self) {
        self.working_dir = absolutize(&self.working_dir);
        self.report_dir = absolutize(&self.report_dir);
        self.sub_dir = absolutize(&self.sub_dir);
        self.config_path = absolutize(&self.config_path);
    }
}

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
    #[setting(env = "NO_COLOR")]
    pub no_color: bool,
    #[setting(cli_only)]
    pub exec: bool,
    #[setting(skip)]
    pub config_path: PathBuf,
    #[setting(flatten)]
    pub commands: Commands,
}

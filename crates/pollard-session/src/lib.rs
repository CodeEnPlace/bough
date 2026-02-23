mod config;

pub use config::{discover_config, read_config, ConfigError, SEARCH_PATHS};
pub use pollard_core::io::{DiffStyle, Render, Style, color, hashed_path};

use pollard_core::config::{LanguageId, Ordering, Vcs, VcsKind};
use pollard_session_derive::Settings;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn compose_vcs(kind: VcsKind, target: Option<String>) -> Vcs {
    match kind {
        VcsKind::None => Vcs::None,
        VcsKind::Git => Vcs::Git {
            commit: target.unwrap_or_else(|| "HEAD".to_string()),
        },
        VcsKind::Jj => Vcs::Jj {
            rev: target.unwrap_or_else(|| "@".to_string()),
        },
        VcsKind::Mercurial => Vcs::Mercurial,
    }
}

fn default_run_id() -> String {
    let seed = format!(
        "{}:{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time went backwards")
            .as_nanos(),
    );
    pollard_core::Hash::of(&seed).to_string()
}

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

pub trait Report: Render {
    fn get_dir(&self, session: &Session) -> PathBuf;
    fn make_path(&self, session: &Session) -> PathBuf;
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
    #[setting(compose(
        fields(kind: VcsKind, target: Option<String>),
        via = "compose_vcs"
    ))]
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
    #[setting(default = "Style::Pretty")]
    pub style: Style,
    #[setting(default = "DiffStyle::Unified")]
    pub diff: DiffStyle,
    #[setting(env = "NO_COLOR")]
    pub no_color: bool,
    #[setting(cli_only, default = "default_run_id()")]
    pub run_id: String,
    #[setting(cli_only)]
    pub exec: bool,
    #[setting(skip)]
    pub config_path: PathBuf,
    #[setting(flatten)]
    pub commands: Commands,
}

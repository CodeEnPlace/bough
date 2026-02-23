mod config;

pub use config::{ConfigError, SEARCH_PATHS, discover_config, read_config};
pub use bough_core::io::{DiffStyle, Render, Style, color, hashed_path};

use bough_core::config::{Ordering, Vcs, VcsKind};
use bough_session_derive::Settings;
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
        self.directories.working = absolutize(&self.directories.working);
        self.directories.state = absolutize(&self.directories.state);
        self.config_path = absolutize(&self.config_path);
    }
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
pub struct Directories {
    #[setting(long = "working-dir")]
    pub working: PathBuf,
    #[setting(long = "state-dir")]
    pub state: PathBuf,
}

#[derive(Debug, Serialize, Settings)]
pub struct Session {
    #[setting(compose(
        fields(kind: VcsKind, target: Option<String>),
        via = "compose_vcs"
    ))]
    pub vcs: Vcs,
    #[setting(flatten)]
    pub directories: Directories,
    pub parallelism: usize,
    pub ordering: Ordering,
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
    #[setting(cli_only)]
    pub exec: bool,
    #[setting(skip)]
    pub config_path: PathBuf,
    #[setting(flatten)]
    pub commands: Commands,
}

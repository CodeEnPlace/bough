use bough_core::config::Config;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::phase_runner::PhaseRunner;
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    NoInitPhase,
    WorkspaceNotFound(PathBuf),
    Phase(crate::phase_runner::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::NoInitPhase => write!(f, "no init phase configured for runner"),
            Error::WorkspaceNotFound(p) => write!(f, "workspace not found: {}", p.display()),
            Error::Phase(e) => write!(f, "init phase failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct InitWorkspace {
    pub workspace: PathBuf,
    pub stdout: String,
}

pub fn run(config: &Config, workspace: &Path) -> Result<InitWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner = config.runner(runner_name);
    let init = config.runner_init_phase(runner_name).ok_or(Error::NoInitPhase)?;

    if !workspace.exists() {
        return Err(Error::WorkspaceNotFound(workspace.to_path_buf()));
    }

    let output = PhaseRunner::new(config, runner, init, workspace)
        .run()
        .map_err(Error::Phase)?;

    Ok(InitWorkspace {
        workspace: workspace.to_path_buf(),
        stdout: output.stdout,
    })
}

impl Render for InitWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "initialized workspace {}\n",
            color("\x1b[1m", &self.workspace.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = format!(
            "initialized workspace {}\n",
            color("\x1b[1m", &self.workspace.display().to_string()),
        );
        if !self.stdout.is_empty() {
            out.push_str(&color("\x1b[2m", &self.stdout));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let mut out = format!(
            "{heading} Init Workspace\n\n- **Path:** `{}`\n",
            self.workspace.display(),
        );
        if !self.stdout.is_empty() {
            out.push_str(&format!("\n```\n{}\n```\n", self.stdout.trim()));
        }
        out
    }
}

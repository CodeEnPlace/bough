use bough_core::config::Config;
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::phase_runner::PhaseRunner;
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    NoTestPhase,
    WorkspaceNotFound(PathBuf),
    Phase(crate::phase_runner::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::NoTestPhase => write!(f, "no test phase configured for runner"),
            Error::WorkspaceNotFound(p) => write!(f, "workspace not found: {}", p.display()),
            Error::Phase(e) => write!(f, "test phase failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct TestWorkspace {
    pub workspace: PathBuf,
    pub stdout: String,
}

pub fn run(config: &Config, workspace: &Path) -> Result<TestWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let test = config.runner_test_phase(runner_name).ok_or(Error::NoTestPhase)?;

    if !workspace.exists() {
        return Err(Error::WorkspaceNotFound(workspace.to_path_buf()));
    }

    let output = PhaseRunner::new(test, workspace)
        .run()
        .map_err(Error::Phase)?;

    Ok(TestWorkspace {
        workspace: workspace.to_path_buf(),
        stdout: output.stdout,
    })
}

impl Render for TestWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "tested workspace {}\n",
            color("\x1b[1m", &self.workspace.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = format!(
            "tested workspace {}\n",
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
            "{heading} Test Workspace\n\n- **Path:** `{}`\n",
            self.workspace.display(),
        );
        if !self.stdout.is_empty() {
            out.push_str(&format!("\n```\n{}\n```\n", self.stdout.trim()));
        }
        out
    }
}

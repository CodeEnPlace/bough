use bough_core::config::{Config, VcsConfig};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::phase_runner::PhaseRunner;
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    WorkspaceNotFound(PathBuf),
    VcsReset(String, i32),
    VcsCommand(String, std::io::Error),
    Phase(crate::phase_runner::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::WorkspaceNotFound(p) => write!(f, "workspace not found: {}", p.display()),
            Error::VcsReset(cmd, code) => write!(f, "vcs reset `{cmd}` exited with code {code}"),
            Error::VcsCommand(cmd, e) => write!(f, "failed to run `{cmd}`: {e}"),
            Error::Phase(e) => write!(f, "reset phase failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct ResetWorkspace {
    pub workspace: PathBuf,
    pub stdout: String,
}

fn vcs_reset(vcs: &VcsConfig, workspace: &Path) -> Result<String, Error> {
    let (cmd, args): (&str, Vec<&str>) = match vcs {
        VcsConfig::Jj { rev } => ("jj", vec!["edit", rev]),
        VcsConfig::Git { commit } => ("git", vec!["checkout", commit, "--force"]),
        VcsConfig::None => return Ok(String::new()),
        VcsConfig::Mercurial => todo!("mercurial workspace reset"),
    };

    let output = Command::new(cmd)
        .args(&args)
        .current_dir(workspace)
        .output()
        .map_err(|e| Error::VcsCommand(format!("{cmd} {}", args.join(" ")), e))?;

    if !output.status.success() {
        return Err(Error::VcsReset(
            format!("{cmd} {}", args.join(" ")),
            output.status.code().unwrap_or(-1),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub fn run(config: &Config, workspace: &Path) -> Result<ResetWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner = config.runner(runner_name);

    if !workspace.exists() {
        return Err(Error::WorkspaceNotFound(workspace.to_path_buf()));
    }

    let mut stdout = vcs_reset(config.vcs(), workspace)?;

    if let Some(reset) = config.runner_reset_phase(runner_name) {
        let output = PhaseRunner::new(config, runner, reset, workspace)
            .run()
            .map_err(Error::Phase)?;
        stdout.push_str(&output.stdout);
    }

    Ok(ResetWorkspace {
        workspace: workspace.to_path_buf(),
        stdout,
    })
}

impl Render for ResetWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "reset workspace {}\n",
            color("\x1b[1m", &self.workspace.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = format!(
            "reset workspace {}\n",
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
            "{heading} Reset Workspace\n\n- **Path:** `{}`\n",
            self.workspace.display(),
        );
        if !self.stdout.is_empty() {
            out.push_str(&format!("\n```\n{}\n```\n", self.stdout.trim()));
        }
        out
    }
}

use bough_core::WorkspaceId;
use bough_core::config::{Config, VcsConfig};
use serde::Serialize;
use std::path::Path;
use std::process::Command;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    NotFound(WorkspaceId),
    Command(String, std::io::Error),
    CommandFailed(String, i32),
    RemoveDir(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::NotFound(id) => write!(f, "workspace '{}' not found", id),
            Error::Command(cmd, e) => write!(f, "failed to run `{cmd}`: {e}"),
            Error::CommandFailed(cmd, code) => write!(f, "`{cmd}` exited with code {code}"),
            Error::RemoveDir(e) => write!(f, "failed to remove workspace dir: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct DropWorkspace {
    pub name: WorkspaceId,
    pub vcs: VcsConfig,
}

fn run_cmd(cmd: &str, args: &[&str], cwd: &Path) -> Result<(), Error> {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .map_err(|e| Error::Command(format!("{cmd} {}", args.join(" ")), e))?;
    if !status.success() {
        return Err(Error::CommandFailed(
            format!("{cmd} {}", args.join(" ")),
            status.code().unwrap_or(-1),
        ));
    }
    Ok(())
}

pub fn run(config: &Config, name: &WorkspaceId) -> Result<DropWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner_pwd = config.runner_pwd(runner_name).ok_or(Error::NoActiveRunner)?;
    let vcs = config.vcs().clone();

    match &vcs {
        VcsConfig::Jj { .. } => {
            run_cmd(
                "jj",
                &["workspace", "forget", &name],
                runner_pwd,
            )?;
            let dir = std::path::PathBuf::from(config.working_dir()).join(&name);
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(Error::RemoveDir)?;
            }
        }
        VcsConfig::Git { .. } => {
            run_cmd(
                "git",
                &["worktree", "remove", &name, "--force"],
                runner_pwd,
            )?;
            let _ = run_cmd(
                "git",
                &["branch", "-D", &name],
                runner_pwd,
            );
        }
        VcsConfig::None => {
            let dir = std::path::PathBuf::from(config.working_dir()).join(&name);
            if !dir.exists() {
                return Err(Error::NotFound(name.clone()));
            }
            std::fs::remove_dir_all(&dir).map_err(Error::RemoveDir)?;
        }
        VcsConfig::Mercurial => {
            todo!("mercurial workspace support")
        }
    }

    Ok(DropWorkspace {
        name: name.clone(),
        vcs,
    })
}

impl Render for DropWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!("dropped workspace {}\n", color("\x1b[1m", &self.name))
    }

    fn render_verbose(&self) -> String {
        format!(
            "dropped workspace {} (vcs: {:?})\n",
            color("\x1b[1m", &self.name),
            self.vcs,
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        format!(
            "{heading} Dropped Workspace\n\n- **Name:** `{}`\n- **VCS:** {:?}\n",
            self.name, self.vcs,
        )
    }
}

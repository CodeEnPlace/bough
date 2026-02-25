use bough_core::config::{Config, VcsConfig};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    CreateDir(std::io::Error),
    Command(String, std::io::Error),
    CommandFailed(String, i32),
    Copy(String, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::CreateDir(e) => write!(f, "failed to create workspace dir: {e}"),
            Error::Command(cmd, e) => write!(f, "failed to run `{cmd}`: {e}"),
            Error::CommandFailed(cmd, code) => write!(f, "`{cmd}` exited with code {code}"),
            Error::Copy(detail, e) => write!(f, "copy failed ({detail}): {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct MakeWorkspace {
    pub path: PathBuf,
    pub vcs: VcsConfig,
    pub name: Option<String>,
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

fn generate_name() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("bough-{ts}")
}

pub fn run(config: &Config) -> Result<MakeWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner_pwd = config.runner_pwd(runner_name).ok_or(Error::NoActiveRunner)?;
    let work_dir = PathBuf::from(config.working_dir());
    let vcs = config.vcs().clone();

    std::fs::create_dir_all(&work_dir).map_err(Error::CreateDir)?;

    let name = match &vcs {
        VcsConfig::Jj { .. } => {
            let name = generate_name();
            run_cmd(
                "jj",
                &["workspace", "add", "--name", &name, &work_dir.to_string_lossy()],
                Path::new(runner_pwd),
            )?;
            Some(name)
        }
        VcsConfig::Git { .. } => {
            let name = generate_name();
            run_cmd(
                "git",
                &["worktree", "add", "-b", &name, &work_dir.to_string_lossy()],
                Path::new(runner_pwd),
            )?;
            Some(name)
        }
        VcsConfig::None => {
            run_cmd(
                "cp",
                &["-a", runner_pwd, &work_dir.to_string_lossy()],
                Path::new(runner_pwd),
            )
            .map_err(|e| match e {
                Error::Command(cmd, io) => Error::Copy(cmd, io),
                Error::CommandFailed(cmd, code) => Error::Copy(cmd, std::io::Error::other(format!("exit code {code}"))),
                other => other,
            })?;
            None
        }
        VcsConfig::Mercurial => {
            todo!("mercurial workspace support")
        }
    };

    Ok(MakeWorkspace {
        path: work_dir,
        vcs,
        name,
    })
}

impl Render for MakeWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "created workspace at {}\n",
            color("\x1b[1m", &self.path.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let vcs_label = match &self.vcs {
            VcsConfig::Jj { .. } => "jj workspace",
            VcsConfig::Git { .. } => "git worktree",
            VcsConfig::None => "cp",
            VcsConfig::Mercurial => "mercurial",
        };
        let mut out = format!(
            "created workspace via {} at {}\n",
            color("\x1b[36m", vcs_label),
            color("\x1b[1m", &self.path.display().to_string()),
        );
        if let Some(name) = &self.name {
            out.push_str(&format!("  name: {}\n", color("\x1b[33m", name)));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let mut out = format!(
            "{heading} Workspace\n\n- **VCS:** {:?}\n- **Path:** `{}`\n",
            self.vcs,
            self.path.display(),
        );
        if let Some(name) = &self.name {
            out.push_str(&format!("- **Name:** `{name}`\n"));
        }
        out
    }
}

use bough_core::WorkspaceId;
use bough_core::config::{Config, VcsConfig};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    Command(String, std::io::Error),
    CommandFailed(String, i32),
    Utf8(std::string::FromUtf8Error),
    ReadDir(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::Command(cmd, e) => write!(f, "failed to run `{cmd}`: {e}"),
            Error::CommandFailed(cmd, code) => write!(f, "`{cmd}` exited with code {code}"),
            Error::Utf8(e) => write!(f, "invalid utf8 in command output: {e}"),
            Error::ReadDir(e) => write!(f, "failed to read workspace dir: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Serialize)]
pub struct Workspace {
    pub name: WorkspaceId,
    pub path: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct ListWorkspaces {
    pub workspaces: Vec<Workspace>,
    pub vcs: VcsConfig,
}

fn run_cmd_output(cmd: &str, args: &[&str], cwd: &Path) -> Result<String, Error> {
    let output = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| Error::Command(format!("{cmd} {}", args.join(" ")), e))?;
    if !output.status.success() {
        return Err(Error::CommandFailed(
            format!("{cmd} {}", args.join(" ")),
            output.status.code().unwrap_or(-1),
        ));
    }
    String::from_utf8(output.stdout).map_err(Error::Utf8)
}

pub fn run(config: &Config) -> Result<ListWorkspaces, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner_pwd = config.runner_pwd(runner_name).ok_or(Error::NoActiveRunner)?;
    let vcs = config.vcs().clone();

    let workspaces = match &vcs {
        VcsConfig::Jj { .. } => {
            let out = run_cmd_output("jj", &["workspace", "list"], Path::new(runner_pwd))?;
            out.lines()
                .filter_map(|l| {
                    let name = l.split(':').next()?.trim();
                    name.starts_with("bough-").then_some(WorkspaceId(name.to_string()))
                })
                .map(|name| {
                    let path = PathBuf::from(config.working_dir()).join(&name);
                    Workspace { name, path }
                })
                .collect()
        }
        VcsConfig::Git { .. } => {
            let out = run_cmd_output("git", &["worktree", "list", "--porcelain"], Path::new(runner_pwd))?;
            let mut workspaces = Vec::new();
            let mut current_path: Option<PathBuf> = None;
            for line in out.lines() {
                if let Some(path) = line.strip_prefix("worktree ") {
                    current_path = Some(PathBuf::from(path));
                } else if let Some(branch) = line.strip_prefix("branch refs/heads/") {
                    if branch.starts_with("bough-") {
                        if let Some(path) = current_path.take() {
                            workspaces.push(Workspace {
                                name: WorkspaceId(branch.to_string()),
                                path,
                            });
                        }
                    }
                }
            }
            workspaces
        }
        VcsConfig::None => {
            let work_dir = config.working_dir();
            let entries = std::fs::read_dir(work_dir).map_err(Error::ReadDir)?;
            entries
                .filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.starts_with("bough-").then(|| Workspace {
                        path: e.path(),
                        name: WorkspaceId(name),
                    })
                })
                .collect()
        }
        VcsConfig::Mercurial => {
            todo!("mercurial workspace support")
        }
    };

    Ok(ListWorkspaces { workspaces, vcs })
}

impl Render for ListWorkspaces {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(&self.workspaces).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "{} workspaces\n",
            color("\x1b[1m", &self.workspaces.len().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = color("\x1b[1m", &format!("{} workspaces", self.workspaces.len()));
        out.push('\n');
        for ws in &self.workspaces {
            out.push_str(&format!(
                "  {} {}\n",
                ws.name,
                color("\x1b[2m", &ws.path.display().to_string()),
            ));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let mut out = format!("{heading} Workspaces ({})\n\n", self.workspaces.len());
        out.push_str("| Name | Path |\n");
        out.push_str("|------|------|\n");
        for ws in &self.workspaces {
            out.push_str(&format!("| `{}` | `{}` |\n", ws.name, ws.path.display()));
        }
        out.push('\n');
        out
    }
}

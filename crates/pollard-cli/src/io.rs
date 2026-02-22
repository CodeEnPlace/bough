use pollard_core::{Hash, MutationKind};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::fmt::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, clap::ValueEnum)]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

#[derive(Debug, Clone, Serialize, clap::ValueEnum)]
pub enum Style {
    Plain,
    Pretty,
    Json,
    Markdown,
}

pub trait Report {
    fn render(&self, style: &Style, no_color: bool, depth: u8);
}

pub enum Action {
    WriteFile { path: PathBuf, content: String },
    CreateJjWorkspace { name: String, path: PathBuf },
    ForgetJjWorkspace { name: String },
    RemoveDir { path: PathBuf },
    RemoveFile { path: PathBuf },
}

impl Action {
    pub fn apply(self) -> std::io::Result<()> {
        match self {
            Action::WriteFile { path, content } => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&path, content)
            }
            Action::CreateJjWorkspace { name, path } => {
                log::info!("creating jj workspace {name} at {}", path.display());
                let output = std::process::Command::new("jj")
                    .args([
                        "workspace",
                        "add",
                        "--name",
                        &name,
                        &path.display().to_string(),
                    ])
                    .output()?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("jj workspace add failed: {stderr}"),
                    ))
                } else {
                    Ok(())
                }
            }
            Action::ForgetJjWorkspace { name } => {
                log::info!("forgetting jj workspace {name}");
                let output = std::process::Command::new("jj")
                    .args(["workspace", "forget", &name])
                    .output()?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("jj workspace forget failed: {stderr}"),
                    ))
                } else {
                    Ok(())
                }
            }
            Action::RemoveDir { path } => {
                log::info!("removing dir {}", path.display());
                std::fs::remove_dir_all(&path)
            }
            Action::RemoveFile { path } => {
                log::info!("removing file {}", path.display());
                std::fs::remove_file(&path)
            }
        }
    }
}

pub fn repo_is_dirty() -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .map(|out| !out.status.success() || !out.stdout.is_empty())
        .unwrap_or(true)
}

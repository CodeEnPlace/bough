use crate::Hash;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Plain,
    Pretty,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

pub trait Render {
    fn render_json(&self) -> String;
    fn render_pretty(&self, depth: u8) -> String;
    fn render_markdown(&self, depth: u8) -> String;

    fn render(&self, style: &Style, no_color: bool, depth: u8) {
        let output = match style {
            Style::Json => self.render_json(),
            Style::Markdown => self.render_markdown(depth),
            Style::Pretty if no_color => strip_ansi(&self.render_pretty(depth)),
            Style::Pretty => self.render_pretty(depth),
            Style::Plain => strip_ansi(&self.render_pretty(depth)),
        };
        print!("{output}");
    }
}

pub fn color(code: &str, text: &str) -> String {
    format!("{code}{text}\x1b[0m")
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn hashed_path(dir: &PathBuf, content: &str, label: &str) -> PathBuf {
    dir.join(format!("{}.{label}.json", Hash::of(content)))
}

#[derive(Debug, Clone, Serialize)]
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
            Action::RemoveDir { path } => std::fs::remove_dir_all(&path),
            Action::RemoveFile { path } => std::fs::remove_file(&path),
        }
    }
}

impl Render for Action {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        match self {
            Action::WriteFile { path, .. } => {
                format!("write {}\n", color("\x1b[36m", &path.display().to_string()))
            }
            Action::CreateJjWorkspace { name, path } => {
                format!(
                    "jj workspace add {} at {}\n",
                    color("\x1b[33m", name),
                    color("\x1b[36m", &path.display().to_string()),
                )
            }
            Action::ForgetJjWorkspace { name } => {
                format!("jj workspace forget {}\n", color("\x1b[33m", name))
            }
            Action::RemoveDir { path } => {
                format!("rm -r {}\n", color("\x1b[36m", &path.display().to_string()))
            }
            Action::RemoveFile { path } => {
                format!("rm {}\n", color("\x1b[36m", &path.display().to_string()))
            }
        }
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

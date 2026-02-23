use serde::Serialize;
use std::path::PathBuf;

pub use pollard_core::io::{DiffStyle, Render, Style, color, hashed_path};
pub use pollard_session::{Report, Session};

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
    fn render(&self, style: &Style, no_color: bool, _depth: u8) {
        let no_color = no_color || matches!(style, Style::Plain);
        match style {
            Style::Json => {
                println!(
                    "{}",
                    serde_json::to_string(self).expect("failed to serialize")
                );
            }
            Style::Plain | Style::Pretty | Style::Markdown => match self {
                Action::WriteFile { path, .. } => {
                    println!(
                        "write {}",
                        color("\x1b[36m", &path.display().to_string(), no_color)
                    );
                }
                Action::CreateJjWorkspace { name, path } => {
                    println!(
                        "jj workspace add {} at {}",
                        color("\x1b[33m", name, no_color),
                        color("\x1b[36m", &path.display().to_string(), no_color),
                    );
                }
                Action::ForgetJjWorkspace { name } => {
                    println!("jj workspace forget {}", color("\x1b[33m", name, no_color));
                }
                Action::RemoveDir { path } => {
                    println!(
                        "rm -r {}",
                        color("\x1b[36m", &path.display().to_string(), no_color)
                    );
                }
                Action::RemoveFile { path } => {
                    println!(
                        "rm {}",
                        color("\x1b[36m", &path.display().to_string(), no_color)
                    );
                }
            },
        }
    }
}

impl Report for Action {
    fn get_dir(&self, session: &Session) -> PathBuf {
        session.report_dir.join("action")
    }

    fn make_path(&self, session: &Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "action")
    }
}

pub struct SessionReport {
    json: String,
    debug: String,
}

impl SessionReport {
    pub fn new(session: &Session) -> Self {
        Self {
            json: serde_json::to_string_pretty(session).expect("failed to serialize"),
            debug: format!("{session:#?}"),
        }
    }
}

impl Render for SessionReport {
    fn render(&self, style: &Style, _no_color: bool, _depth: u8) {
        match style {
            Style::Json => println!("{}", self.json),
            _ => println!("{}", self.debug),
        }
    }
}

impl Report for SessionReport {
    fn get_dir(&self, session: &Session) -> PathBuf {
        session.report_dir.join("session")
    }

    fn make_path(&self, session: &Session) -> PathBuf {
        hashed_path(&self.get_dir(session), &self.json, "session")
    }
}

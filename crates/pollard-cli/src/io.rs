use serde::Serialize;
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

fn color(code: &str, text: &str, no_color: bool) -> String {
    if no_color {
        text.to_string()
    } else {
        format!("{code}{text}\x1b[0m")
    }
}

impl Report for Action {
    fn render(&self, style: &Style, no_color: bool, _depth: u8) {
        match style {
            Style::Json => {
                println!(
                    "{}",
                    serde_json::to_string(self).expect("failed to serialize")
                );
            }
            Style::Pretty => match self {
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
            Style::Plain | Style::Markdown => match self {
                Action::WriteFile { path, .. } => println!("write {}", path.display()),
                Action::CreateJjWorkspace { name, path } => {
                    println!("jj workspace add {} at {}", name, path.display());
                }
                Action::ForgetJjWorkspace { name } => println!("jj workspace forget {}", name),
                Action::RemoveDir { path } => println!("rm -r {}", path.display()),
                Action::RemoveFile { path } => println!("rm {}", path.display()),
            },
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

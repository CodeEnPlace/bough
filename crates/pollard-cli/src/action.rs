use std::path::PathBuf;

pub enum Action {
    WriteFile { path: PathBuf, content: String },
    CreateJjWorkspace { name: String, path: PathBuf },
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
                    .args(["workspace", "add", "--name", &name, &path.display().to_string()])
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

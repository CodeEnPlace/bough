use std::path::PathBuf;

pub enum Action {
    WriteFile { path: PathBuf, content: String },
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

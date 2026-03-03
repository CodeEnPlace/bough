use crate::config;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug)]
pub enum ValidationError {
    WorkspaceNotFound { name: String, path: PathBuf },
    NoActiveSuite,
    Glob(glob::PatternError),
    ReadFile(PathBuf, std::io::Error),
    CreateDir(PathBuf, std::io::Error),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound { name, path } => {
                write!(f, "workspace '{}' not found at {}", name, path.display())
            }
            Self::NoActiveSuite => write!(f, "no active suite configured"),
            Self::Glob(e) => write!(f, "invalid glob pattern: {e}"),
            Self::ReadFile(p, e) => write!(f, "failed to read {}: {e}", p.display()),
            Self::CreateDir(p, e) => write!(f, "failed to create {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(name: impl Into<String>, config: &config::Config) -> Result<Self, ValidationError> {
        let name = name.into();
        let path = config.bough_dir.join(&name);
        if !path.is_dir() {
            return Err(ValidationError::WorkspaceNotFound { name, path });
        }
        Ok(Self(name))
    }

    pub fn create(
        name: impl Into<String>,
        config: &config::Config,
    ) -> Result<Self, ValidationError> {
        let name = name.into();
        let path = config.bough_dir.join(&name);
        if !path.is_dir() {
            std::fs::create_dir_all(&path).map_err(|e| ValidationError::CreateDir(path, e))?;
        }
        Ok(Self(name))
    }

    pub fn from_trusted(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for WorkspaceId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<std::path::Path> for WorkspaceId {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }
}

pub struct Workspace {}

impl Workspace {}

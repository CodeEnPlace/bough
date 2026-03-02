use std::collections::HashMap;

use crate::WorkspaceId;
use crate::config::Config;
use crate::workspace::Workspace;

#[derive(Debug)]
pub enum Error {
    Discovery(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Discovery(e) => write!(f, "failed to discover workspaces: {e}"),
        }
    }
}

impl std::error::Error for Error {}

pub struct Session {
    config: Config,
    workspaces: HashMap<WorkspaceId, Workspace>,
}

impl Session {
    pub fn new(config: Config) -> Result<Self, Error> {
        let workspaces = Self::discover_workspaces(&config)?;
        Ok(Self { config, workspaces })
    }

    // core[impl session.workspace.discovery]
    fn discover_workspaces(config: &Config) -> Result<HashMap<WorkspaceId, Workspace>, Error> {
        let dir = config.workspaces_dir();
        if !dir.is_dir() {
            return Ok(HashMap::new());
        }
        let mut workspaces = HashMap::new();
        for entry in std::fs::read_dir(&dir).map_err(Error::Discovery)? {
            let entry = entry.map_err(Error::Discovery)?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                workspaces.insert(WorkspaceId::from_trusted(name), Workspace {});
            }
        }
        Ok(workspaces)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;
    use std::path::PathBuf;
    use tempfile::TempDir;

    const BASE_CONFIG: &str = r#"
[test]
command = "npx vitest run"

[mutate.ts]
files.include = ["src/**/*.ts"]
"#;

    fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let dest = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_dir_recursive(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }

    fn setup() -> TempDir {
        let tmp = TempDir::new().unwrap();
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/vitest-js");
        copy_dir_recursive(&src, tmp.path());
        tmp
    }

    fn build_config(source_dir: PathBuf) -> Config {
        let v: toml::Value = toml::from_str(BASE_CONFIG).unwrap();
        ConfigBuilder::new(source_dir)
            .from_value(v)
            .build()
            .unwrap()
    }

    // core[verify session.workspace.discovery]
    #[test]
    fn no_workspaces_dir_yields_empty() {
        let tmp = setup();
        let config = build_config(tmp.path().to_path_buf());
        let session = Session::new(config).unwrap();
        assert!(session.workspaces.is_empty());
    }

    // core[verify session.workspace.discovery]
    #[test]
    fn discovers_existing_workspaces() {
        let tmp = setup();
        let ws_dir = tmp.path().join(".bough/workspaces");
        std::fs::create_dir_all(ws_dir.join("ws-a")).unwrap();
        std::fs::create_dir_all(ws_dir.join("ws-b")).unwrap();
        std::fs::write(ws_dir.join("not-a-dir.txt"), "").unwrap();

        let config = build_config(tmp.path().to_path_buf());
        let session = Session::new(config).unwrap();

        assert_eq!(session.workspaces.len(), 2);
        assert!(
            session
                .workspaces
                .contains_key(&WorkspaceId::from_trusted("ws-a"))
        );
        assert!(
            session
                .workspaces
                .contains_key(&WorkspaceId::from_trusted("ws-b"))
        );
    }
}

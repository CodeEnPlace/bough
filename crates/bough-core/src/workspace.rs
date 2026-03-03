use crate::base::Base;
use crate::file::{Error, FilesIter, Root};
use std::path::{Path, PathBuf};

// core[impl workspace.root]
// core[impl workspace.base]
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace<'a> {
    root: PathBuf,
    base: &'a Base,
}

impl<'a> Workspace<'a> {
    pub fn new(root: PathBuf, base: &'a Base) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self { root, base })
    }

    pub fn base(&self) -> &Base {
        self.base
    }

    // core[impl workspace.files]
    pub fn files(&self) -> FilesIter {
        self.base.files()
    }
}

impl Root for Workspace<'_> {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileSourceConfig;

    // core[verify workspace.root]
    #[test]
    fn workspace_impls_root() {
        let base = Base::new(PathBuf::from("/tmp/base"), FileSourceConfig::default()).unwrap();
        let ws = Workspace::new(PathBuf::from("/tmp/ws"), &base).unwrap();
        assert_eq!(ws.path(), Path::new("/tmp/ws"));
    }

    // core[verify workspace.root]
    #[test]
    fn workspace_rejects_relative_path() {
        let base = Base::new(PathBuf::from("/tmp/base"), FileSourceConfig::default()).unwrap();
        assert!(matches!(
            Workspace::new(PathBuf::from("relative"), &base),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }

    // core[verify workspace.base]
    #[test]
    fn workspace_holds_base() {
        let base = Base::new(PathBuf::from("/tmp/base"), FileSourceConfig::default()).unwrap();
        let ws = Workspace::new(PathBuf::from("/tmp/ws"), &base).unwrap();
        assert_eq!(ws.base().path(), Path::new("/tmp/base"));
    }

    // core[verify workspace.files]
    #[test]
    fn workspace_files_returns_iter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            FileSourceConfig {
                include: vec!["*.txt".into()],
                ..Default::default()
            },
        )
        .unwrap();
        let ws = Workspace::new(PathBuf::from("/tmp/ws"), &base).unwrap();
        let twigs: Vec<_> = ws.files().collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }
}

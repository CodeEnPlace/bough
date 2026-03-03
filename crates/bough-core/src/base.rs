use crate::config::FileSourceConfig;
use crate::file::{Error, FilesIter, Root};
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files_config: FileSourceConfig,
}

impl Base {
    pub fn new(root: PathBuf, files_config: FileSourceConfig) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self { root, files_config })
    }

    // core[impl base.files]
    pub fn files(&self) -> FilesIter {
        FilesIter::new(self, &self.files_config)
    }
}

impl Root for Base {
    fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // core[verify base.root]
    #[test]
    fn base_impls_root() {
        let base = Base::new(PathBuf::from("/tmp/project"), FileSourceConfig::default()).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    // core[verify base.root]
    #[test]
    fn base_rejects_relative_path() {
        assert!(matches!(
            Base::new(PathBuf::from("relative"), FileSourceConfig::default()),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }

    // core[verify base.files]
    #[test]
    fn base_files_returns_iter() {
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
        let twigs: Vec<_> = base.files().collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }
}

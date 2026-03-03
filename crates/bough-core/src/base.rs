use crate::config::{FileSourceConfig, LanguageId};
use crate::file::{Error, FilesIter, Root};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files_config: FileSourceConfig,
    mutant_files_configs: HashMap<LanguageId, FileSourceConfig>,
}

impl Base {
    pub fn new(root: PathBuf, files_config: FileSourceConfig) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            files_config,
            mutant_files_configs: HashMap::new(),
        })
    }

    pub fn with_mutant_files_configs(
        root: PathBuf,
        files_config: FileSourceConfig,
        mutant_files_configs: HashMap<LanguageId, FileSourceConfig>,
    ) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            files_config,
            mutant_files_configs,
        })
    }

    // core[impl base.files]
    pub fn files(&self) -> FilesIter {
        FilesIter::new(self, &self.files_config)
    }

    // core[impl base.mutant_files]
    pub fn mutant_files(&self, language_id: &LanguageId) -> FilesIter {
        match self.mutant_files_configs.get(language_id) {
            Some(config) => FilesIter::new(self, config),
            None => FilesIter::new(self, &FileSourceConfig::default()),
        }
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

    // core[verify base.mutant_files]
    #[test]
    fn base_mutant_files_returns_language_specific_iter() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "js").unwrap();
        std::fs::write(dir.path().join("src/b.ts"), "ts").unwrap();

        let mut lang_configs = std::collections::HashMap::new();
        lang_configs.insert(
            crate::config::LanguageId::Javascript,
            FileSourceConfig {
                include: vec!["src/**/*.js".into()],
                ..Default::default()
            },
        );
        lang_configs.insert(
            crate::config::LanguageId::Typescript,
            FileSourceConfig {
                include: vec!["src/**/*.ts".into()],
                ..Default::default()
            },
        );

        let base = Base::with_mutant_files_configs(
            dir.path().to_path_buf(),
            FileSourceConfig {
                include: vec!["src/**/*".into()],
                ..Default::default()
            },
            lang_configs,
        )
        .unwrap();

        let js_twigs: Vec<_> = base
            .mutant_files(&crate::config::LanguageId::Javascript)
            .collect();
        assert_eq!(js_twigs.len(), 1);
        assert_eq!(js_twigs[0].path(), Path::new("src/a.js"));

        let ts_twigs: Vec<_> = base
            .mutant_files(&crate::config::LanguageId::Typescript)
            .collect();
        assert_eq!(ts_twigs.len(), 1);
        assert_eq!(ts_twigs[0].path(), Path::new("src/b.ts"));
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

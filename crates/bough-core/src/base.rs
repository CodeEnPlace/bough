use crate::LanguageId;
use crate::file::{Error, FilesIter, Root, Twig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    twigs: Vec<Twig>,
    mutant_twigs: HashMap<LanguageId, Vec<Twig>>,
}

impl Base {
    pub fn new(
        root: PathBuf,
        files: FilesIter,
    ) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            twigs: files.collect(),
            mutant_twigs: HashMap::new(),
        })
    }

    pub fn with_mutant_files(
        root: PathBuf,
        files: FilesIter,
        mutant_files: HashMap<LanguageId, FilesIter>,
    ) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            twigs: files.collect(),
            mutant_twigs: mutant_files.into_iter().map(|(k, v)| (k, v.collect())).collect(),
        })
    }

    // core[impl base.files]
    pub fn files(&self) -> impl Iterator<Item = Twig> + '_ {
        self.twigs.iter().cloned()
    }

    // core[impl base.mutant_files]
    pub fn mutant_files(&self, language_id: &LanguageId) -> impl Iterator<Item = Twig> + '_ {
        self.mutant_twigs
            .get(language_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
            .iter()
            .cloned()
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

    use crate::file::FilesIter;

    fn files_for(root: &Path, include: &[String]) -> FilesIter {
        FilesIter::new(root, include, &[], &[])
    }

    // core[verify base.root]
    #[test]
    fn base_impls_root() {
        let base = Base::new(
            PathBuf::from("/tmp/project"),
            files_for(Path::new("/tmp/project"), &[]),
        ).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    // core[verify base.root]
    #[test]
    fn base_rejects_relative_path() {
        assert!(matches!(
            Base::new(PathBuf::from("relative"), files_for(Path::new("relative"), &[])),
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

        let root = dir.path();
        let mut lang_files = std::collections::HashMap::new();
        lang_files.insert(
            crate::LanguageId::Javascript,
            files_for(root, &["src/**/*.js".into()]),
        );
        lang_files.insert(
            crate::LanguageId::Typescript,
            files_for(root, &["src/**/*.ts".into()]),
        );

        let base = Base::with_mutant_files(
            root.to_path_buf(),
            files_for(root, &["src/**/*".into()]),
            lang_files,
        )
        .unwrap();

        let js_twigs: Vec<_> = base
            .mutant_files(&crate::LanguageId::Javascript)
            .collect();
        assert_eq!(js_twigs.len(), 1);
        assert_eq!(js_twigs[0].path(), Path::new("src/a.js"));

        let ts_twigs: Vec<_> = base
            .mutant_files(&crate::LanguageId::Typescript)
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
            files_for(dir.path(), &["*.txt".into()]),
        )
        .unwrap();
        let twigs: Vec<_> = base.files().collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }
}

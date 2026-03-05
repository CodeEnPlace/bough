use crate::LanguageId;
use crate::file::{Error, TwigsIter, Root, Twig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: TwigsIter,
    mutant_files: HashMap<LanguageId, TwigsIter>,
}

impl Base {
    pub fn new(
        root: PathBuf,
        files: TwigsIter,
    ) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            files,
            mutant_files: HashMap::new(),
        })
    }

    pub fn add_mutator(&mut self, language_id: LanguageId, files: TwigsIter) {
        self.mutant_files.insert(language_id, files);
    }

    // core[impl base.files]
    pub fn files(&self) -> impl Iterator<Item = Twig> + '_ {
        self.files.iter()
    }

    // core[impl base.mutant_files]
    pub fn mutant_files(&self, language_id: &LanguageId) -> impl Iterator<Item = Twig> + '_ {
        self.mutant_files
            .get(language_id)
            .into_iter()
            .flat_map(|f| f.iter())
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

    use crate::file::TwigsIter;

    fn files_for(root: &Path, include: &[String]) -> TwigsIter {
        TwigsIter::new(root, include, &[], &[])
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
        let mut base = Base::new(
            root.to_path_buf(),
            files_for(root, &["src/**/*".into()]),
        )
        .unwrap();
        base.add_mutator(crate::LanguageId::Javascript, files_for(root, &["src/**/*.js".into()]));
        base.add_mutator(crate::LanguageId::Typescript, files_for(root, &["src/**/*.ts".into()]));

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

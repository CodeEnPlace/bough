use bough_core::{LanguageId, TwigsIterBuilder};
use bough_fs::{Error, Root, Twig};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: BTreeSet<Twig>,
    mutant_files: HashMap<LanguageId, (BTreeSet<Twig>, Vec<String>)>,
}

/// Base is meant for *scanning* the base directory, and to act as a handle for it.
/// It's not for storing state or making decisions — that's `Session`'s job.
impl Base {
    pub fn new(root: PathBuf, files: TwigsIterBuilder) -> Result<Self, Error> {
        bough_fs::validate_root(&root)?;
        debug!(root = %root.display(), "created base");
        let mut base = Self {
            root,
            files: BTreeSet::new(),
            mutant_files: HashMap::new(),
        };
        base.files = files.build(&base).collect();
        Ok(base)
    }

    pub fn add_mutator(
        &mut self,
        language_id: LanguageId,
        files: TwigsIterBuilder,
        skip_queries: Vec<String>,
    ) {
        debug!(lang = ?language_id, skip_queries = ?skip_queries, "added mutator");
        let twigs: BTreeSet<Twig> = files.build(self).collect();
        self.mutant_files.insert(language_id, (twigs, skip_queries));
    }

    pub fn twigs<'a>(&'a self) -> impl Iterator<Item = Twig> + 'a {
        self.files.iter().cloned()
    }

    pub fn mutant_twigs(&self) -> impl Iterator<Item = (LanguageId, Twig)> + '_ {
        self.mutant_files
            .iter()
            .flat_map(|(language_id, (twigs, _))| {
                twigs.iter().map(move |twig| (*language_id, twig.clone()))
            })
    }

    pub fn skip_queries_for(&self, language_id: LanguageId) -> &[String] {
        self.mutant_files
            .get(&language_id)
            .map(|(_, q)| q.as_slice())
            .unwrap_or(&[])
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
    use bough_core::TwigsIterBuilder;

    fn files_for(include: &[&str]) -> TwigsIterBuilder {
        let mut builder = TwigsIterBuilder::new();
        for glob in include {
            builder = builder.with_include_glob(glob);
        }
        builder
    }

    #[test]
    #[cfg(unix)]
    fn base_impls_root() {
        let base = Base::new(PathBuf::from("/tmp/project"), files_for(&[])).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    #[test]
    fn twigs_are_computed_once_at_construction() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "").unwrap();
        std::fs::write(dir.path().join("b.txt"), "").unwrap();
        let base = Base::new(dir.path().to_path_buf(), files_for(&["*.txt"])).unwrap();
        let before: Vec<_> = base.twigs().collect();
        std::fs::remove_file(dir.path().join("a.txt")).unwrap();
        std::fs::remove_file(dir.path().join("b.txt")).unwrap();
        let after: Vec<_> = base.twigs().collect();
        assert_eq!(before, after);
        assert_eq!(before.len(), 2);
    }

    #[test]
    fn mutant_twigs_are_computed_once_at_add_mutator() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.js"), "").unwrap();
        let mut base = Base::new(dir.path().to_path_buf(), files_for(&[])).unwrap();
        base.add_mutator(LanguageId::Javascript, files_for(&["*.js"]), vec![]);
        let before: Vec<_> = base.mutant_twigs().collect();
        std::fs::remove_file(dir.path().join("a.js")).unwrap();
        let after: Vec<_> = base.mutant_twigs().collect();
        assert_eq!(before, after);
        assert_eq!(before.len(), 1);
    }

    #[test]
    fn base_rejects_relative_path() {
        let err = Base::new(PathBuf::from("relative"), files_for(&[])).unwrap_err();
        match err {
            Error::RootMustBeAbsolute(_) => {}
            other => panic!("expected RootMustBeAbsolute, got {other:?}"),
        }
    }
}

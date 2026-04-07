use bough_core::{LanguageId, TwigsIterBuilder};
use bough_fs::{Error, Root, Twig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::debug;

#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: TwigsIterBuilder,
    mutant_files: HashMap<LanguageId, (TwigsIterBuilder, Vec<String>)>,
}

/// Base is meant for *scanning* the base directory, and to act as a handle for it.
/// It's not for storing state or making decisions — that's `Session`'s job.
impl Base {
    pub fn new(root: PathBuf, files: TwigsIterBuilder) -> Result<Self, Error> {
        bough_fs::validate_root(&root)?;
        debug!(root = %root.display(), "created base");
        Ok(Self {
            root,
            files,
            mutant_files: HashMap::new(),
        })
    }

    pub fn add_mutator(
        &mut self,
        language_id: LanguageId,
        files: TwigsIterBuilder,
        skip_queries: Vec<String>,
    ) {
        debug!(lang = ?language_id, skip_queries = ?skip_queries, "added mutator");
        self.mutant_files.insert(language_id, (files, skip_queries));
    }

    pub fn twigs<'a>(&'a self) -> impl Iterator<Item = Twig> + 'a {
        self.files.clone().build(self)
    }

    pub fn mutant_twigs(&self) -> impl Iterator<Item = (LanguageId, Twig)> + '_ {
        self.mutant_files
            .iter()
            .flat_map(|(language_id, (twigs_iter_builder, _))| {
                twigs_iter_builder
                    .clone()
                    .build(self)
                    .map(|twig| (language_id.clone(), twig))
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
    fn base_impls_root() {
        let base = Base::new(PathBuf::from("/tmp/project"), files_for(&[])).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
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

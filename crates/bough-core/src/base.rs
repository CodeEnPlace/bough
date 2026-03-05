use crate::LanguageId;
use crate::file::{Error, Root, Twig};
use crate::mutant::{BasedMutant, TwigMutantsIter};
use crate::twig::{TwigsIter, TwigsIterBuilder};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// core[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: TwigsIterBuilder,
    mutant_files: HashMap<LanguageId, TwigsIterBuilder>,
}

impl Base {
    pub fn new(root: PathBuf, files: TwigsIterBuilder) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        Ok(Self {
            root,
            files,
            mutant_files: HashMap::new(),
        })
    }

    pub fn add_mutator(&mut self, language_id: LanguageId, files: TwigsIterBuilder) {
        self.mutant_files.insert(language_id, files);
    }

    pub fn twigs<'a>(&'a self) -> impl Iterator<Item = Twig> + 'a {
        self.files.clone().build(self)
    }

    pub fn mutant_twigs(&self) -> impl Iterator<Item = (LanguageId, Twig)> + '_ {
        self.mutant_files
            .iter()
            .flat_map(|(language_id, twigs_iter_builder)| {
                twigs_iter_builder
                    .clone()
                    .build(self)
                    .map(|twig| (language_id.clone(), twig))
            })
    }

    pub fn mutants(&self) -> impl Iterator<Item = std::io::Result<BasedMutant<'_>>> + '_ {
        self.mutant_twigs().flat_map(|(language_id, twig)| {
            match TwigMutantsIter::new(language_id, self, &twig) {
                Ok(iter) => iter.map(Ok).collect::<Vec<_>>(),
                Err(e) => vec![Err(e)],
            }
        })
    }

    pub fn mutations(&self) -> impl Iterator<Item = std::io::Result<Mutation<'_>>> + '_ {
        self.mutant_twigs().flat_map(|(language_id, twig)| {
            match TwigMutantsIter::new(language_id, self, &twig) {
                Ok(iter) => iter.map(Ok).collect::<Vec<_>>(),
                Err(e) => vec![Err(e)],
            }
        })
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
    use crate::file::TestRoot;
    use crate::twig::TwigsIterBuilder;

    fn files_for(root: &impl Root, include: &[&str]) -> TwigsIterBuilder {
        let mut builder = TwigsIterBuilder::new();
        for glob in include {
            builder = builder.with_include_glob(glob);
        }
        builder
    }

    // core[verify base.root]
    #[test]
    fn base_impls_root() {
        let root = TestRoot::new("/tmp/project");
        let base = Base::new(PathBuf::from("/tmp/project"), files_for(&root, &[])).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    // core[verify base.root]
    #[test]
    fn base_rejects_relative_path() {
        let root = TestRoot::new("relative");
        assert!(matches!(
            Base::new(PathBuf::from("relative"), files_for(&root, &[])),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }
}

use crate::LanguageId;
use crate::file::{Error, Root, Twig};
use crate::mutant::{Mutant, TwigMutantsIter};
use crate::mutation::{Mutation, MutationIter};
use crate::twig::TwigsIterBuilder;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

// bough[impl base.root]
#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: TwigsIterBuilder,
    mutant_files: HashMap<LanguageId, TwigsIterBuilder>,
}

/// Base is meant for *scanning* the base directory, and to act as a handle for it
/// It's not for storing state or making decitions, that's [Session]'s job
impl Base {
    pub fn new(root: PathBuf, files: TwigsIterBuilder) -> Result<Self, Error> {
        crate::file::validate_root(&root)?;
        debug!(root = %root.display(), "created base");
        Ok(Self {
            root,
            files,
            mutant_files: HashMap::new(),
        })
    }

    pub fn add_mutator(&mut self, language_id: LanguageId, files: TwigsIterBuilder) {
        debug!(lang = ?language_id, "added mutator");
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

    pub fn mutants(&self) -> impl Iterator<Item = std::io::Result<Mutant>> + '_ {
        self.mutant_twigs().flat_map(|(language_id, twig)| {
            trace!(lang = ?language_id, twig = %twig.path().display(), "scanning twig for mutants");
            match TwigMutantsIter::new(language_id, self, &twig) {
                Ok(iter) => iter.map(|bm| Ok(bm.into_mutant())).collect::<Vec<_>>(),
                Err(e) => {
                    warn!(lang = ?language_id, twig = %twig.path().display(), error = %e, "failed to scan twig for mutants");
                    vec![Err(e)]
                }
            }
        })
    }

    pub fn mutations(&self) -> impl Iterator<Item = std::io::Result<Mutation>> + '_ {
        self.mutants().flat_map(|r| match r {
            Ok(m) => MutationIter::new(&m).map(Ok).collect::<Vec<_>>(),
            Err(e) => vec![Err(e)],
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
    use crate::twig::TwigsIterBuilder;

    fn files_for(include: &[&str]) -> TwigsIterBuilder {
        let mut builder = TwigsIterBuilder::new();
        for glob in include {
            builder = builder.with_include_glob(glob);
        }
        builder
    }

    // bough[verify base.root]
    #[test]
    fn base_impls_root() {
        let base = Base::new(PathBuf::from("/tmp/project"), files_for(&[])).unwrap();
        assert_eq!(base.path(), Path::new("/tmp/project"));
    }

    // bough[verify base.root]
    #[test]
    fn base_rejects_relative_path() {
        assert!(matches!(
            Base::new(PathBuf::from("relative"), files_for(&[])),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }
}

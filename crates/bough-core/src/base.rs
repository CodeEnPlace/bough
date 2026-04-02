use crate::LanguageId;
use crate::mutant::{Mutant, TwigMutantsIter};
use crate::mutation::{Mutation, MutationIter};
use bough_fs::TwigsIterBuilder;
use bough_fs::{Error, Root, Twig};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, PartialEq)]
pub struct Base {
    root: PathBuf,
    files: TwigsIterBuilder,
    mutant_files: HashMap<LanguageId, (TwigsIterBuilder, Vec<String>)>,
}

/// Base is meant for *scanning* the base directory, and to act as a handle for it
/// It's not for storing state or making decitions, that's [Session]'s job
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

    fn skip_queries_for(&self, language_id: LanguageId) -> &[String] {
        self.mutant_files
            .get(&language_id)
            .map(|(_, q)| q.as_slice())
            .unwrap_or(&[])
    }

    pub fn mutants(&self) -> impl Iterator<Item = std::io::Result<Mutant>> + '_ {
        self.mutant_twigs().flat_map(|(language_id, twig)| {
            trace!(lang = ?language_id, twig = %twig.path().display(), "scanning twig for mutants");
            match TwigMutantsIter::new(language_id, self, &twig) {
                Ok(iter) => {
                    let iter = self.skip_queries_for(language_id).iter().fold(iter, |iter, q| {
                        iter.with_skip_query(q)
                    });
                    iter.map(|bm| Ok(bm.into_mutant())).collect::<Vec<_>>()
                }
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

impl Base {
    pub fn run_test(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        crate::phase::run_phase_in_base(
            self,
            &config.get_test_cmd(),
            config.get_test_pwd(),
            config.get_test_env(),
            Some(config.get_test_timeout(reference_duration)),
        )
    }

    pub fn run_init(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        let cmd = config
            .get_init_cmd()
            .ok_or(crate::phase::Error::NoCmdConfigured)?;
        crate::phase::run_phase_in_base(
            self,
            &cmd,
            config.get_init_pwd(),
            config.get_init_env(),
            Some(config.get_init_timeout(reference_duration)),
        )
    }

    pub fn run_reset(
        &self,
        config: &impl crate::session::Config,
        reference_duration: Option<chrono::Duration>,
    ) -> Result<crate::phase::PhaseOutcome, crate::phase::Error> {
        let cmd = config
            .get_reset_cmd()
            .ok_or(crate::phase::Error::NoCmdConfigured)?;
        crate::phase::run_phase_in_base(
            self,
            &cmd,
            config.get_reset_pwd(),
            config.get_reset_env(),
            Some(config.get_reset_timeout(reference_duration)),
        )
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
    use bough_fs::TwigsIterBuilder;

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
        assert!(matches!(
            Base::new(PathBuf::from("relative"), files_for(&[])),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }
}

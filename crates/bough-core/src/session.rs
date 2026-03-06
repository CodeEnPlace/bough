use std::{collections::HashSet, path::PathBuf};

use bough_typed_hash::TypedHashable;

use crate::{
    LanguageId, base::Base, facet_disk_store::FacetDiskStore, mutation::Mutation, state::State,
    twig::TwigsIterBuilder,
};

trait Config {
    fn get_bough_state_dir(&self) -> PathBuf;
    fn get_base_root_path(&self) -> PathBuf;
    fn get_base_include_globs(&self) -> impl Iterator<Item = &str>;
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str>;

    fn get_langs(&self) -> impl Iterator<Item = LanguageId>;
    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
}

pub enum Error {
    File(crate::file::Error),
    Io(std::io::Error),
}

pub struct TestConfig {}

pub struct Session<C: Config> {
    config: C,
    base: Base,
    workspaces: Vec<Base>,
    mutations_in_base: HashSet<Mutation>,
}

impl<C: Config> Session<C> {
    // core[impl session.init]
    pub fn new(config: C) -> Result<Self, Error> {
        let workspaces = vec![];

        let mut base_twigs_iter_builder = TwigsIterBuilder::new();
        for include in config.get_base_include_globs() {
            base_twigs_iter_builder = base_twigs_iter_builder.with_include_glob(include);
        }
        for exclude in config.get_base_exclude_globs() {
            base_twigs_iter_builder = base_twigs_iter_builder.with_exclude_glob(exclude);
        }

        let mut base = Base::new(config.get_base_root_path(), base_twigs_iter_builder)?;

        for lang in config.get_langs() {
            let mut lang_twigs_iter_builder = TwigsIterBuilder::new();
            for include in config.get_lang_include_globs(lang) {
                lang_twigs_iter_builder = lang_twigs_iter_builder.with_include_glob(include);
            }
            for exclude in config.get_lang_exclude_globs(lang) {
                lang_twigs_iter_builder = lang_twigs_iter_builder.with_exclude_glob(exclude);
            }
            base.add_mutator(lang, lang_twigs_iter_builder);
        }

        let mutations_in_base = base.mutations().collect::<Result<HashSet<_>, _>>()?;
        let mutations_state = FacetDiskStore::<<Mutation as TypedHashable>::Hash, State>::new(
            config.get_bough_state_dir().join("state"),
        );

        Ok(Self {
            config,
            base,
            workspaces,
            mutations_in_base,
        })
    }
}

impl From<crate::file::Error> for Error {
    fn from(e: crate::file::Error) -> Self {
        Error::File(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MinimalConfig {
        root: PathBuf,
    }

    impl Config for MinimalConfig {
        fn get_base_root_path(&self) -> PathBuf {
            self.root.clone()
        }

        fn get_base_include_globs(&self) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_lang_include_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_lang_exclude_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_langs(&self) -> impl Iterator<Item = LanguageId> {
            vec![].into_iter()
        }

        fn get_bough_state_dir(&self) -> PathBuf {
            todo!()
        }
    }

    // core[verify session.init]
    #[test]
    fn session_new_creates_session_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: dir.path().to_path_buf(),
        };
        let session = Session::new(config);
        assert!(session.is_ok());
    }

    // core[verify session.init]
    #[test]
    fn session_new_fails_with_invalid_root() {
        let config = MinimalConfig {
            root: PathBuf::from("not/absolute"),
        };
        let session = Session::new(config);
        assert!(session.is_err());
    }
}

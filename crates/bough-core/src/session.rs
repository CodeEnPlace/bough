use std::path::PathBuf;

use crate::{LanguageId, base::Base, file::Error, twig::TwigsIterBuilder};

trait Config {
    fn get_base_root_path(&self) -> PathBuf;
    fn get_base_include_globs(&self) -> impl Iterator<Item = &str>;
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str>;

    fn get_langs(&self) -> impl Iterator<Item = LanguageId>;
    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
}

pub struct TestConfig {}

pub struct Session<C: Config> {
    config: C,
    base: Base,
    workspaces: Vec<Base>,
}

impl<C: Config> Session<C> {
    // core[impl session.init]
    pub fn new(config: C) -> Result<Self, Error> {
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

        let workspaces = vec![];

        Ok(Self {
            config,
            base,
            workspaces,
        })
    }
}

impl Config for TestConfig {
    fn get_base_root_path(&self) -> PathBuf {
        todo!()
    }

    fn get_base_include_globs(&self) -> impl Iterator<Item = &str> {
        vec![].into_iter()
    }

    fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str> {
        vec![].into_iter()
    }

    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str> {
        vec![].into_iter()
    }

    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str> {
        vec![].into_iter()
    }

    fn get_langs(&self) -> impl Iterator<Item = LanguageId> {
        vec![].into_iter()
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

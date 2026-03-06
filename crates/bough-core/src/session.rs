use std::{collections::HashSet, path::PathBuf};

use bough_typed_hash::TypedHashable;

use crate::{
    LanguageId,
    base::Base,
    facet_disk_store::FacetDiskStore,
    mutation::{Mutation, MutationHash},
    state::State,
    twig::TwigsIterBuilder,
};

trait Config {
    fn get_workers_count(&self) -> u64;

    fn get_bough_state_dir(&self) -> PathBuf;
    fn get_base_root_path(&self) -> PathBuf;
    fn get_base_include_globs(&self) -> impl Iterator<Item = &str>;
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str>;

    fn get_langs(&self) -> impl Iterator<Item = LanguageId>;
    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = &str>;
}

#[derive(Debug)]
pub enum Error {
    File(crate::file::Error),
    Io(std::io::Error),
}

pub struct TestConfig {}

pub struct Session<C: Config> {
    config: C,
    base: Base,
    workspaces: Vec<Base>,
    mutations_state: FacetDiskStore<MutationHash, State>,
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

        let mutations_in_base: HashSet<Mutation> = base.mutations().collect::<Result<_, _>>()?;

        // core[impl session.init.state.attach]
        let mut mutations_state = FacetDiskStore::<<Mutation as TypedHashable>::Hash, State>::new(
            config.get_bough_state_dir().join("state"),
        );

        let mut hash_store = bough_typed_hash::MemoryHashStore::new();

        let hashes_in_base: HashSet<MutationHash> = mutations_in_base
            .iter()
            .map(|m| m.hash(&mut hash_store).expect("hashing should not fail"))
            .collect();

        // core[impl session.init.state.add-missing]
        for mutation in &mutations_in_base {
            let hash = mutation
                .hash(&mut hash_store)
                .expect("hashing should not fail");
            if mutations_state.get(&hash).is_none() {
                let state = State::new(mutation.clone());
                mutations_state
                    .set(hash, state)
                    .expect("writing state should not fail");
            }
        }

        // core[impl session.init.state.remove-stale]
        let stale_keys: Vec<MutationHash> = mutations_state
            .keys()
            .filter(|k| !hashes_in_base.contains(k))
            .collect();
        for key in stale_keys {
            mutations_state.remove(&key);
        }

        Ok(Self {
            config,
            base,
            workspaces,
            mutations_state,
        })
    }

    // core[impl session.init.state.get]
    pub fn get_state(&self) -> &FacetDiskStore<MutationHash, State> {
        &self.mutations_state
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
        state_dir: PathBuf,
        langs: Vec<LanguageId>,
        lang_includes: Vec<String>,
    }

    impl Config for MinimalConfig {
        fn get_base_root_path(&self) -> PathBuf {
            self.root.clone()
        }

        fn get_base_include_globs(&self) -> impl Iterator<Item = &str> {
            self.lang_includes.iter().map(|s| s.as_str())
        }

        fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_lang_include_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = &str> {
            self.lang_includes.iter().map(|s| s.as_str())
        }

        fn get_lang_exclude_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = &str> {
            vec![].into_iter()
        }

        fn get_langs(&self) -> impl Iterator<Item = LanguageId> {
            self.langs.clone().into_iter()
        }

        fn get_bough_state_dir(&self) -> PathBuf {
            self.state_dir.clone()
        }

        fn get_workers_count(&self) -> u64 {
            todo!()
        }
    }

    fn make_js_config(dir: &std::path::Path) -> MinimalConfig {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        MinimalConfig {
            root: dir.to_path_buf(),
            state_dir: dir.join("bough"),
            langs: vec![LanguageId::Javascript],
            lang_includes: vec!["src/**/*.js".to_string()],
        }
    }

    // core[verify session.init]
    #[test]
    fn session_new_creates_session_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: dir.path().to_path_buf(),
            state_dir: dir.path().join("state"),
            langs: vec![],
            lang_includes: vec![],
        };
        let session = Session::new(config);
        assert!(session.is_ok());
    }

    // core[verify session.init]
    #[test]
    fn session_new_fails_with_invalid_root() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: PathBuf::from("not/absolute"),
            state_dir: dir.path().join("state"),
            langs: vec![],
            lang_includes: vec![],
        };
        let session = Session::new(config);
        assert!(session.is_err());
    }

    fn state_dir(dir: &std::path::Path) -> PathBuf {
        dir.join("bough/state")
    }

    fn state_files(dir: &std::path::Path) -> Vec<PathBuf> {
        let sd = state_dir(dir);
        if !sd.exists() {
            return vec![];
        }
        let mut files: Vec<PathBuf> = std::fs::read_dir(&sd)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect();
        files.sort();
        files
    }

    // core[verify session.init.state.attach]
    #[test]
    fn session_creates_state_files_under_config_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let _session = Session::new(config).unwrap();
        let sd = state_dir(dir.path());
        assert!(sd.exists(), "state dir {sd:?} should exist on disk");
        let files = state_files(dir.path());
        assert!(!files.is_empty(), "state dir should contain json files");
        for f in &files {
            assert!(
                f.starts_with(&sd),
                "state file {f:?} should be under {sd:?}"
            );
        }
    }

    // core[verify session.init.state.get]
    #[test]
    fn session_get_state_reads_back_disk_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let session = Session::new(config).unwrap();
        let state = session.get_state();
        let disk_count = state_files(dir.path()).len();
        let api_count = state.keys().count();
        assert_eq!(api_count, disk_count);
        for key in state.keys() {
            assert!(state.get(&key).is_some());
        }
    }

    // core[verify session.init.state.add-missing]
    #[test]
    fn session_adds_missing_mutations_as_state_files_with_null_outcome() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let session = Session::new(config).unwrap();
        let files = state_files(dir.path());
        assert_eq!(files.len(), 1);
        let content = std::fs::read_to_string(&files[0]).unwrap();
        assert_eq!(
            content,
            r#"{"mutation":{"mutant":{"lang":"js","twig":["src/a.js"],"kind":"StatementBlock","span":{"start":{"line":0,"col":15,"byte":15},"end":{"line":0,"col":28,"byte":28}}},"subst":"{}"},"outcome":null}"#,
        );
        let state = session.get_state();
        let key = state.keys().next().unwrap();
        let s = state.get(&key).unwrap();
        assert!(!s.has_outcome());
        assert_eq!(s.mutation().subst(), "{}");
    }

    // core[verify session.init.state.remove-stale]
    #[test]
    fn session_removes_stale_state_files_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let sd = state_dir(dir.path());
        std::fs::create_dir_all(&sd).unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();

        let stale_content = r#"{"mutation":{"mutant":{"lang":"js","twig":["old/gone.js"],"kind":"StatementBlock","span":{"start":{"line":0,"col":0,"byte":0},"end":{"line":0,"col":1,"byte":1}}},"subst":"{}"},"outcome":null}"#;
        let fake_hash_a = "aa".repeat(32);
        let fake_hash_b = "bb".repeat(32);
        std::fs::write(sd.join(format!("{fake_hash_a}.json")), stale_content).unwrap();
        std::fs::write(sd.join(format!("{fake_hash_b}.json")), stale_content).unwrap();
        assert_eq!(state_files(dir.path()).len(), 2);

        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let _session = Session::new(config).unwrap();

        let files_after = state_files(dir.path());
        assert_eq!(
            files_after.len(),
            0,
            "stale state files should be removed, but found: {files_after:?}"
        );
    }
}

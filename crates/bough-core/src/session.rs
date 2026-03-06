use std::{collections::HashSet, path::PathBuf};

use bough_typed_hash::TypedHashable;
use tracing::{debug, info, warn};

use crate::{
    LanguageId,
    base::Base,
    facet_disk_store::FacetDiskStore,
    mutation::{Mutation, MutationHash},
    state::State,
    twig::TwigsIterBuilder,
    workspace::{Workspace, WorkspaceId},
};

pub trait Config {
    fn get_workers_count(&self) -> u64;

    fn get_bough_state_dir(&self) -> PathBuf;
    fn get_base_root_path(&self) -> PathBuf;
    fn get_base_include_globs(&self) -> impl Iterator<Item = String>;
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = String>;

    fn get_langs(&self) -> impl Iterator<Item = LanguageId>;
    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = String>;
    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = String>;
}

#[derive(Debug)]
pub enum Error {
    File(crate::file::Error),
    Io(std::io::Error),
    Workspace(crate::workspace::Error),
}

pub struct TestConfig {}

pub struct Session<C: Config> {
    config: C,
    base: Base,
    workspaces: Vec<WorkspaceId>,
    mutations_state: FacetDiskStore<MutationHash, State>,
}

impl<C: Config> Session<C> {
    // bough[impl session.init+2]
    pub fn new(config: C) -> Result<Self, Error> {
        info!("initializing session");
        let workspaces_dir = config.get_bough_state_dir().join("workspaces");

        // bough[impl session.init.workspaces.bind]
        debug!(dir = %workspaces_dir.display(), "scanning for existing workspaces");
        let existing_ids: Vec<WorkspaceId> = if workspaces_dir.join("work").exists() {
            std::fs::read_dir(workspaces_dir.join("work"))?
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    WorkspaceId::parse(&name).ok()
                })
                .collect()
        } else {
            vec![]
        };

        let mut base_twigs_iter_builder = TwigsIterBuilder::new();
        for include in config.get_base_include_globs() {
            base_twigs_iter_builder = base_twigs_iter_builder.with_include_glob(&include);
        }
        for exclude in config.get_base_exclude_globs() {
            base_twigs_iter_builder = base_twigs_iter_builder.with_exclude_glob(&exclude);
        }

        let mut base = Base::new(config.get_base_root_path(), base_twigs_iter_builder)?;

        for lang in config.get_langs() {
            let mut lang_twigs_iter_builder = TwigsIterBuilder::new();
            for include in config.get_lang_include_globs(lang) {
                lang_twigs_iter_builder = lang_twigs_iter_builder.with_include_glob(&include);
            }
            for exclude in config.get_lang_exclude_globs(lang) {
                lang_twigs_iter_builder = lang_twigs_iter_builder.with_exclude_glob(&exclude);
            }
            base.add_mutator(lang, lang_twigs_iter_builder);
        }

        debug!("search for mutations in base");
        let mutations_in_base: HashSet<Mutation> = base.mutations().collect::<Result<_, _>>()?;
        debug!(
            count = mutations_in_base.len(),
            "discovered mutations in base"
        );

        // bough[impl session.init.state.attach]
        let mut mutations_state = FacetDiskStore::<<Mutation as TypedHashable>::Hash, State>::new(
            config.get_bough_state_dir().join("state"),
        );

        let mut hash_store = bough_typed_hash::MemoryHashStore::new();

        let hashes_in_base: HashSet<MutationHash> = mutations_in_base
            .iter()
            .map(|m| m.hash(&mut hash_store).expect("hashing should not fail"))
            .collect();

        // bough[impl session.init.state.add-missing]
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

        // bough[impl session.init.state.remove-stale]
        let stale_keys: Vec<MutationHash> = mutations_state
            .keys()
            .filter(|k| !hashes_in_base.contains(k))
            .collect();
        if !stale_keys.is_empty() {
            info!(count = stale_keys.len(), "removing stale state entries");
        }
        for key in stale_keys {
            mutations_state.remove(&key);
        }

        // bough[impl session.init.workspaces]
        let mut workspaces: Vec<WorkspaceId> = Vec::new();

        for id in &existing_ids {
            Workspace::bind(workspaces_dir.clone(), id, &base)?;
            workspaces.push(id.clone());
        }

        let needed = (config.get_workers_count() as usize).saturating_sub(workspaces.len());
        if needed > 0 {
            debug!(count = needed, "creating new workspaces");
        }
        for _ in 0..needed {
            let ws = Workspace::new(workspaces_dir.clone(), &base)?;
            workspaces.push(ws.id().clone());
        }

        info!(
            workspaces = workspaces.len(),
            mutations = mutations_state.keys().count(),
            "session initialized"
        );

        Ok(Self {
            config,
            base,
            workspaces,
            mutations_state,
        })
    }

    pub fn base(&self) -> &Base {
        &self.base
    }

    // bough[impl session.tend.state.add-missing]
    pub fn tend_add_missing_states(&mut self) -> Result<Vec<MutationHash>, Error> {
        let mutations_in_base: HashSet<Mutation> =
            self.base.mutations().collect::<Result<_, _>>()?;
        let mut hash_store = bough_typed_hash::MemoryHashStore::new();
        let mut added = Vec::new();

        for mutation in &mutations_in_base {
            let hash = mutation
                .hash(&mut hash_store)
                .expect("hashing should not fail");
            if self.mutations_state.get(&hash).is_none() {
                let state = State::new(mutation.clone());
                self.mutations_state
                    .set(hash.clone(), state)
                    .expect("writing state should not fail");
                added.push(hash);
            }
        }

        Ok(added)
    }

    // bough[impl session.tend.state.remove-stale]
    pub fn tend_remove_stale_states(&mut self) -> Result<Vec<MutationHash>, Error> {
        let mutations_in_base: HashSet<Mutation> =
            self.base.mutations().collect::<Result<_, _>>()?;
        let mut hash_store = bough_typed_hash::MemoryHashStore::new();

        let hashes_in_base: HashSet<MutationHash> = mutations_in_base
            .iter()
            .map(|m| m.hash(&mut hash_store).expect("hashing should not fail"))
            .collect();

        let stale_keys: Vec<MutationHash> = self
            .mutations_state
            .keys()
            .filter(|k| !hashes_in_base.contains(k))
            .collect();

        for key in &stale_keys {
            self.mutations_state.remove(key);
        }

        Ok(stale_keys)
    }

    // bough[impl session.init.state.get]
    pub fn get_state(&self) -> &FacetDiskStore<MutationHash, State> {
        &self.mutations_state
    }

    // bough[impl session.init.workspaces.get-ids]
    pub fn workspace_ids(&self) -> &[WorkspaceId] {
        &self.workspaces
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

impl From<crate::workspace::Error> for Error {
    fn from(e: crate::workspace::Error) -> Self {
        Error::Workspace(e)
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
        workers_count: u64,
    }

    impl Config for MinimalConfig {
        fn get_base_root_path(&self) -> PathBuf {
            self.root.clone()
        }

        fn get_base_include_globs(&self) -> impl Iterator<Item = String> {
            self.lang_includes.clone().into_iter()
        }

        fn get_base_exclude_globs(&self) -> impl Iterator<Item = String> {
            Vec::<String>::new().into_iter()
        }

        fn get_lang_include_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = String> {
            self.lang_includes.clone().into_iter()
        }

        fn get_lang_exclude_globs(&self, _language_id: LanguageId) -> impl Iterator<Item = String> {
            Vec::<String>::new().into_iter()
        }

        fn get_langs(&self) -> impl Iterator<Item = LanguageId> {
            self.langs.clone().into_iter()
        }

        fn get_bough_state_dir(&self) -> PathBuf {
            self.state_dir.clone()
        }

        fn get_workers_count(&self) -> u64 {
            self.workers_count
        }
    }

    fn make_js_config(dir: &std::path::Path) -> MinimalConfig {
        std::fs::create_dir_all(dir.join("src")).unwrap();
        MinimalConfig {
            root: dir.to_path_buf(),
            state_dir: dir.join("bough"),
            langs: vec![LanguageId::Javascript],
            lang_includes: vec!["src/**/*.js".to_string()],
            workers_count: 0,
        }
    }

    // bough[verify session.init+2]
    #[test]
    fn session_new_creates_session_from_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: dir.path().to_path_buf(),
            state_dir: dir.path().join("state"),
            langs: vec![],
            lang_includes: vec![],
            workers_count: 0,
        };
        let session = Session::new(config);
        assert!(session.is_ok());
    }

    // bough[verify session.init+2]
    #[test]
    fn session_new_fails_with_invalid_root() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: PathBuf::from("not/absolute"),
            state_dir: dir.path().join("state"),
            langs: vec![],
            lang_includes: vec![],
            workers_count: 0,
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

    // bough[verify session.init.state.attach]
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

    // bough[verify session.init.state.get]
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

    // bough[verify session.init.state.add-missing]
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

    fn workspace_dirs(dir: &std::path::Path) -> Vec<PathBuf> {
        let ws_dir = dir.join("bough/workspaces/work");
        if !ws_dir.exists() {
            return vec![];
        }
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(&ws_dir)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        dirs
    }

    // bough[verify session.init.workspaces]
    #[test]
    fn session_creates_workspaces_in_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut config = make_js_config(dir.path());
        config.workers_count = 3;
        let _session = Session::new(config).unwrap();
        let dirs = workspace_dirs(dir.path());
        assert_eq!(dirs.len(), 3, "expected 3 workspaces, got: {dirs:?}");
        for d in &dirs {
            assert!(
                d.join("src/a.js").exists(),
                "workspace should contain source files"
            );
        }
    }

    // bough[verify session.init.workspaces.bind]
    #[test]
    fn session_binds_existing_workspaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut config = make_js_config(dir.path());
        config.workers_count = 2;
        let _session = Session::new(config).unwrap();
        let dirs_before = workspace_dirs(dir.path());
        assert_eq!(dirs_before.len(), 2);

        let mut config2 = make_js_config(dir.path());
        config2.workers_count = 2;
        let _session2 = Session::new(config2).unwrap();
        let dirs_after = workspace_dirs(dir.path());
        assert_eq!(
            dirs_after.len(),
            2,
            "should reuse existing workspaces, not create new ones"
        );
        assert_eq!(dirs_before, dirs_after, "workspace dirs should be the same");
    }

    // bough[verify session.init.workspaces.get-ids]
    #[test]
    fn session_workspace_ids_returns_created_ids() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut config = make_js_config(dir.path());
        config.workers_count = 3;
        let session = Session::new(config).unwrap();
        let ids = session.workspace_ids();
        assert_eq!(ids.len(), 3);
        for id in ids {
            assert_eq!(id.as_str().len(), 8);
        }
    }

    // bough[verify session.tend.state.add-missing]
    #[test]
    fn tend_add_missing_states_adds_new_mutations() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        let initial_count = session.get_state().keys().count();
        assert!(initial_count > 0);

        std::fs::write(
            dir.path().join("src/b.js"),
            "function bar() { return 2; }",
        )
        .unwrap();

        let mut config2 = make_js_config(dir.path());
        config2.lang_includes = vec!["src/**/*.js".to_string()];
        let mut session2 = Session::new(config2).unwrap();
        let count_after = session2.get_state().keys().count();
        assert!(count_after > initial_count);

        let added = session2.tend_add_missing_states().unwrap();
        assert!(added.is_empty(), "no new mutations since session2 was created fresh");
    }

    // bough[verify session.tend.state.add-missing]
    #[test]
    fn tend_add_missing_states_returns_newly_added_hashes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        let initial_count = session.get_state().keys().count();

        std::fs::write(
            dir.path().join("src/b.js"),
            "function bar() { return 2; }",
        )
        .unwrap();
        session.base.add_mutator(
            LanguageId::Javascript,
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        );

        let added = session.tend_add_missing_states().unwrap();
        assert!(!added.is_empty());
        let final_count = session.get_state().keys().count();
        assert_eq!(final_count, initial_count + added.len());
    }

    // bough[verify session.tend.state.remove-stale]
    #[test]
    fn tend_remove_stale_states_removes_orphaned_entries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        std::fs::write(dir.path().join("src/b.js"), "function bar() { return 2; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        let initial_count = session.get_state().keys().count();
        assert!(initial_count > 0);

        std::fs::remove_file(dir.path().join("src/b.js")).unwrap();
        session.base = crate::base::Base::new(
            dir.path().to_path_buf(),
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        session.base.add_mutator(
            LanguageId::Javascript,
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        );

        let removed = session.tend_remove_stale_states().unwrap();
        assert!(!removed.is_empty());
        let final_count = session.get_state().keys().count();
        assert!(final_count < initial_count);
        assert_eq!(final_count + removed.len(), initial_count);
    }

    // bough[verify session.tend.state.remove-stale]
    #[test]
    fn tend_remove_stale_states_returns_empty_when_nothing_stale() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();

        let removed = session.tend_remove_stale_states().unwrap();
        assert!(removed.is_empty());
    }

    // bough[verify session.init.state.remove-stale]
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

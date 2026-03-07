use std::{collections::HashSet, path::PathBuf};

use bough_typed_hash::TypedHashable;
use tracing::{debug, info};

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
        let mutations_state = FacetDiskStore::<<Mutation as TypedHashable>::Hash, State>::new(
            config.get_bough_state_dir().join("state"),
        );

        let mut workspaces: Vec<WorkspaceId> = Vec::new();

        for id in &existing_ids {
            Workspace::bind(workspaces_dir.clone(), id, &base)?;
            workspaces.push(id.clone());
        }

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

    // bough[impl session.tend.workspaces]
    // bough[impl session.tend.workspaces.bind]
    // bough[impl session.tend.workspaces.bind.validate-unchanged.rm]
    // bough[impl session.tend.workspaces.bind.validate-unchanged.forget]
    // bough[impl session.tend.workspaces.new]
    // bough[impl session.tend.workspaces.surplus]
    pub fn tend_workspaces(&mut self, desired_count: usize) -> Result<Vec<WorkspaceId>, Error> {
        let workspaces_dir = self.config.get_bough_state_dir().join("workspaces");

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

        let mut valid: Vec<WorkspaceId> = Vec::new();
        for id in &existing_ids {
            match Workspace::bind(workspaces_dir.clone(), id, &self.base) {
                Ok(_ws) => valid.push(id.clone()),
                Err(_) => {
                    let ws_path = workspaces_dir.join("work").join(id.as_str());
                    if ws_path.exists() {
                        std::fs::remove_dir_all(&ws_path)?;
                    }
                }
            }
        }

        if valid.len() > desired_count {
            for id in valid.drain(desired_count..) {
                let ws_path = workspaces_dir.join("work").join(id.as_str());
                if ws_path.exists() {
                    std::fs::remove_dir_all(&ws_path)?;
                }
            }
        }

        while valid.len() < desired_count {
            let ws = Workspace::new(workspaces_dir.clone(), &self.base)?;
            valid.push(ws.id().clone());
        }

        self.workspaces = valid.clone();
        Ok(valid)
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

    // bough[verify session.init.state.attach]
    #[test]
    fn session_new_creates_state_dir() {
        let dir = tempfile::tempdir().unwrap();
        let config = MinimalConfig {
            root: dir.path().to_path_buf(),
            state_dir: dir.path().join("my-state"),
            langs: vec![],
            lang_includes: vec![],
            workers_count: 0,
        };
        let _session = Session::new(config).unwrap();
        assert!(
            dir.path().join("my-state/state").exists(),
            "mutations_state dir should be created at <state_dir>/state"
        );
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
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(3).unwrap();
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
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(2).unwrap();
        let dirs_before = workspace_dirs(dir.path());
        assert_eq!(dirs_before.len(), 2);

        let config2 = make_js_config(dir.path());
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
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(3).unwrap();
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
        session.tend_add_missing_states().unwrap();
        let initial_count = session.get_state().keys().count();
        assert!(initial_count > 0);

        std::fs::write(dir.path().join("src/b.js"), "function bar() { return 2; }").unwrap();

        let mut config2 = make_js_config(dir.path());
        config2.lang_includes = vec!["src/**/*.js".to_string()];
        let mut session2 = Session::new(config2).unwrap();
        session2.tend_add_missing_states().unwrap();
        let count_after = session2.get_state().keys().count();
        assert!(count_after > initial_count);

        let added = session2.tend_add_missing_states().unwrap();
        assert!(
            added.is_empty(),
            "no new mutations since session2 was created fresh"
        );
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

        std::fs::write(dir.path().join("src/b.js"), "function bar() { return 2; }").unwrap();
        session.base.add_mutator(
            LanguageId::Javascript,
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        );

        let added = session.tend_add_missing_states().unwrap();
        assert!(!added.is_empty());
        let final_count = session.get_state().keys().count();
        assert_eq!(final_count, initial_count + added.len());
    }

    // bough[verify session.tend.workspaces]
    #[test]
    fn tend_workspaces_returns_workspace_ids() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut config = make_js_config(dir.path());
        config.workers_count = 0;
        let mut session = Session::new(config).unwrap();
        let ids = session.tend_workspaces(3).unwrap();
        assert_eq!(ids.len(), 3);
    }

    // bough[verify session.tend.workspaces.bind]
    #[test]
    fn tend_workspaces_binds_existing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(2).unwrap();
        let dirs_before = workspace_dirs(dir.path());
        assert_eq!(dirs_before.len(), 2);

        let ids = session.tend_workspaces(2).unwrap();
        assert_eq!(ids.len(), 2);
        let dirs_after = workspace_dirs(dir.path());
        assert_eq!(dirs_before, dirs_after);
    }

    // bough[verify session.tend.workspaces.bind.validate-unchanged.rm]
    // bough[verify session.tend.workspaces.bind.validate-unchanged.forget]
    #[test]
    fn tend_workspaces_removes_modified_workspace_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(2).unwrap();
        let dirs_before = workspace_dirs(dir.path());
        assert_eq!(dirs_before.len(), 2);

        std::fs::write(dirs_before[0].join("src/a.js"), "MUTATED").unwrap();

        let ids = session.tend_workspaces(2).unwrap();
        assert_eq!(ids.len(), 2);
        assert!(
            !dirs_before[0].exists(),
            "modified workspace dir should be removed from disk"
        );
    }

    // bough[verify session.tend.workspaces.new]
    #[test]
    fn tend_workspaces_creates_new_to_reach_desired_count() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(1).unwrap();
        assert_eq!(workspace_dirs(dir.path()).len(), 1);

        let ids = session.tend_workspaces(3).unwrap();
        assert_eq!(ids.len(), 3);
        assert_eq!(workspace_dirs(dir.path()).len(), 3);
    }

    // bough[verify session.tend.workspaces.surplus]
    #[test]
    fn tend_workspaces_removes_surplus() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_workspaces(4).unwrap();
        assert_eq!(workspace_dirs(dir.path()).len(), 4);

        let ids = session.tend_workspaces(2).unwrap();
        assert_eq!(ids.len(), 2);
        assert_eq!(workspace_dirs(dir.path()).len(), 2);
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
        session.tend_add_missing_states().unwrap();
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
}

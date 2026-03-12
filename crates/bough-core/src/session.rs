use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use bough_typed_hash::TypedHashable;
use chrono::Duration;
use tracing::info;

use crate::{
    Factor, LanguageId, Status,
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
    fn get_lang_skip_queries(&self, language_id: LanguageId) -> impl Iterator<Item = String>;

    fn get_test_cmd(&self) -> String;
    fn get_test_pwd(&self) -> PathBuf;
    fn get_test_env(&self) -> HashMap<String, String>;
    fn get_test_timeout_absolute(&self) -> Option<Duration>;
    fn get_test_timeout_relative(&self) -> Option<f64>;

    fn get_init_cmd(&self) -> Option<String>;
    fn get_init_pwd(&self) -> PathBuf;
    fn get_init_env(&self) -> HashMap<String, String>;
    fn get_init_timeout_absolute(&self) -> Option<Duration>;
    fn get_init_timeout_relative(&self) -> Option<f64>;

    fn get_reset_cmd(&self) -> Option<String>;
    fn get_reset_pwd(&self) -> PathBuf;
    fn get_reset_env(&self) -> HashMap<String, String>;
    fn get_reset_timeout_absolute(&self) -> Option<Duration>;
    fn get_reset_timeout_relative(&self) -> Option<f64>;

    fn get_find_number(&self) -> usize;
    fn get_find_number_per_file(&self) -> usize;
    fn get_find_factors(&self) -> Vec<Factor>;
}

#[derive(Debug)]
pub enum Error {
    File(crate::file::Error),
    Io(std::io::Error),
    Workspace(crate::workspace::Error),
    Phase(crate::phase::Error),
    NoCmdConfigured,
    AbsolutePwd(PathBuf),
    InvalidTimeout,
}

pub struct TestConfig {}

pub struct Session<C: Config> {
    config: C,
    base: Arc<Base>,
    workspaces: Vec<WorkspaceId>,
    mutations_state: FacetDiskStore<MutationHash, State>,
    mutations_needing_test: Vec<MutationHash>,
}

impl<C: Config> Session<C> {
    // bough[impl session.init+2]
    pub fn new(config: C) -> Result<Self, Error> {
        info!("initializing session");
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
            let skip_queries: Vec<String> = config.get_lang_skip_queries(lang).collect();
            base.add_mutator(lang, lang_twigs_iter_builder, skip_queries);
        }

        // bough[impl session.init.state.attach]
        let mutations_state = FacetDiskStore::<<Mutation as TypedHashable>::Hash, State>::new(
            config.get_bough_state_dir().join("state"),
        );

        let mutations_needing_test = Self::derive_mutations_needing_testing(&mutations_state);

        Ok(Self {
            config,
            base: Arc::new(base),
            workspaces: Vec::new(),
            mutations_state,
            mutations_needing_test,
        })
    }

    pub fn base(&self) -> &Base {
        &self.base
    }

    fn derive_mutations_needing_testing(
        mutations_state: &FacetDiskStore<MutationHash, State>,
    ) -> Vec<MutationHash> {
        mutations_state
            .keys()
            .filter(|key| {
                if let Some(val) = mutations_state.get(key) {
                    if let Some(status) = val.status() {
                        if *status == Status::Caught {
                            return false;
                        }
                    }
                };
                return true;
            })
            .collect()
    }

    pub fn get_next_mutation_needing_test(&mut self) -> Option<MutationHash> {
        self.mutations_needing_test.pop()
    }

    pub fn get_count_mutation_needing_test(&mut self) -> usize {
        self.mutations_needing_test.len()
    }

    // bough[impl session.tend.state.add-missing]
    pub fn tend_add_missing_states(&mut self) -> Result<Vec<MutationHash>, Error> {
        let mutations_in_base: HashSet<Mutation> =
            self.base.mutations().collect::<Result<_, _>>()?;
        let mut added = Vec::new();

        for mutation in &mutations_in_base {
            let hash = mutation.hash().expect("hashing should not fail");
            if self.mutations_state.get(&hash).is_none() {
                let state = State::new(mutation.clone());
                self.mutations_state
                    .set(hash.clone(), state)
                    .expect("writing state should not fail");
                added.push(hash);
            }
        }

        self.mutations_needing_test = Self::derive_mutations_needing_testing(&self.mutations_state);

        Ok(added)
    }

    // bough[impl session.tend.state.remove-stale]
    pub fn tend_remove_stale_states(&mut self) -> Result<Vec<MutationHash>, Error> {
        let mutations_in_base: HashSet<Mutation> =
            self.base.mutations().collect::<Result<_, _>>()?;
        let hashes_in_base: HashSet<MutationHash> = mutations_in_base
            .iter()
            .map(|m| m.hash().expect("hashing should not fail"))
            .collect();

        let stale_keys: Vec<MutationHash> = self
            .mutations_state
            .keys()
            .filter(|k| !hashes_in_base.contains(k))
            .collect();

        for key in &stale_keys {
            self.mutations_state.remove(key);
        }

        self.mutations_needing_test = Self::derive_mutations_needing_testing(&self.mutations_state);

        Ok(stale_keys)
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
            match Workspace::bind(workspaces_dir.clone(), id, self.base.clone()) {
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
            let ws = Workspace::new(workspaces_dir.clone(), self.base.clone())?;
            valid.push(ws.id().clone());
        }

        self.workspaces = valid.clone();
        Ok(valid)
    }

    // bough[impl session.init.state.get]
    pub fn get_state(&self) -> &FacetDiskStore<MutationHash, State> {
        &self.mutations_state
    }

    // bough[impl session.init.workspaces.get-ids]
    pub fn workspace_ids(&self) -> &[WorkspaceId] {
        &self.workspaces
    }

    pub fn set_state(
        &mut self,
        mutation: &Mutation,
        status: crate::state::Status,
    ) -> Result<(), Error> {
        let hash = mutation.hash().expect("hashing should not fail");
        let mut state = State::new(mutation.clone());
        state.set_outcome(status);
        self.mutations_state.set(hash, state).map_err(Error::Io)?;
        Ok(())
    }

    pub fn find_best_mutations(&self) -> Result<Vec<(MutationHash, State, f64)>, Error> {
        if self.mutations_needing_test.is_empty() {
            return Ok(Vec::new());
        }

        let factors = self.config.get_find_factors();
        let number = self.config.get_find_number();
        let number_per_file = self.config.get_find_number_per_file();

        let needing_test: Vec<(MutationHash, State)> = self
            .mutations_needing_test
            .iter()
            .filter_map(|hash| {
                let state = self.mutations_state.get(hash)?;
                Some((hash.clone(), state))
            })
            .collect();

        let all_states: Vec<State> = self
            .mutations_state
            .keys()
            .filter_map(|k| self.mutations_state.get(&k))
            .collect();

        let mut scorers: Vec<crate::mutation_score::MutationScorer> = factors
            .iter()
            .map(|f| crate::mutation_score::MutationScorer::new(self.base.clone(), *f))
            .collect();

        let mut scores_per_mutation: Vec<Vec<crate::mutation_score::OpaqueScore>> =
            Vec::with_capacity(needing_test.len());

        for (_, state) in &needing_test {
            let mut mutation_scores = Vec::with_capacity(factors.len());
            for scorer in &mut scorers {
                let score = scorer.score(state.mutation().clone(), &all_states);
                mutation_scores.push(score);
            }
            scores_per_mutation.push(mutation_scores);
        }

        let viewers: Vec<crate::mutation_score::MutationScoreViewer> =
            scorers.into_iter().map(|s| s.into_viewer()).collect();

        let mut scored: Vec<(MutationHash, State, f64)> = needing_test
            .into_iter()
            .zip(scores_per_mutation)
            .map(|((hash, state), scores)| {
                let composite = if viewers.is_empty() {
                    0.0
                } else {
                    let sum: f64 = scores
                        .into_iter()
                        .zip(&viewers)
                        .map(|(s, v)| v.normalize(s))
                        .sum();
                    sum / viewers.len() as f64
                };
                (hash, state, composite)
            })
            .collect();

        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        let mut per_file_counts: HashMap<crate::file::Twig, usize> = HashMap::new();
        scored.retain(|(_, state, _)| {
            let twig = state.mutation().mutant().twig().clone();
            let count = per_file_counts.entry(twig).or_insert(0);
            *count += 1;
            *count <= number_per_file
        });

        scored.truncate(number);

        Ok(scored)
    }

    pub fn bind_workspace(&self, workspace_id: &WorkspaceId) -> Result<Workspace, Error> {
        let workspaces_dir = self.config.get_bough_state_dir().join("workspaces");
        Ok(Workspace::bind(
            workspaces_dir,
            workspace_id,
            self.base.clone(),
        )?)
    }

    pub fn bind_dirty_workspace(&self, workspace_id: &WorkspaceId) -> Workspace {
        let workspaces_dir = self.config.get_bough_state_dir().join("workspaces");
        Workspace::bind_dirty(workspaces_dir, workspace_id, self.base.clone())
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

impl From<crate::phase::Error> for Error {
    fn from(e: crate::phase::Error) -> Self {
        Error::Phase(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct MinimalConfig {
        root: PathBuf,
        state_dir: PathBuf,
        langs: Vec<LanguageId>,
        lang_includes: Vec<String>,
        workers_count: u64,
        test_cmd: String,
        test_pwd: PathBuf,
        init_cmd: Option<String>,
        reset_cmd: Option<String>,
        find_number: usize,
        find_number_per_file: usize,
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

        fn get_lang_skip_queries(&self, _language_id: LanguageId) -> impl Iterator<Item = String> {
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

        fn get_test_cmd(&self) -> String {
            self.test_cmd.clone()
        }

        fn get_test_pwd(&self) -> PathBuf {
            self.test_pwd.clone()
        }

        fn get_test_env(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn get_test_timeout_absolute(&self) -> Option<Duration> {
            None
        }

        fn get_test_timeout_relative(&self) -> Option<f64> {
            None
        }

        fn get_init_cmd(&self) -> Option<String> {
            self.init_cmd.clone()
        }

        fn get_init_pwd(&self) -> PathBuf {
            PathBuf::from(".")
        }

        fn get_init_env(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn get_init_timeout_absolute(&self) -> Option<Duration> {
            None
        }

        fn get_init_timeout_relative(&self) -> Option<f64> {
            None
        }

        fn get_reset_cmd(&self) -> Option<String> {
            self.reset_cmd.clone()
        }

        fn get_reset_pwd(&self) -> PathBuf {
            PathBuf::from(".")
        }

        fn get_reset_env(&self) -> HashMap<String, String> {
            HashMap::new()
        }

        fn get_reset_timeout_absolute(&self) -> Option<Duration> {
            None
        }

        fn get_reset_timeout_relative(&self) -> Option<f64> {
            None
        }

        fn get_find_number(&self) -> usize {
            self.find_number
        }

        fn get_find_number_per_file(&self) -> usize {
            self.find_number_per_file
        }

        fn get_find_factors(&self) -> Vec<Factor> {
            vec![Factor::EncompasingMissedMutationsCount, Factor::TSNodeDepth]
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
            test_cmd: "echo test".to_string(),
            test_pwd: PathBuf::from("."),
            init_cmd: None,
            reset_cmd: None,
            find_number: 1,
            find_number_per_file: 1,
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
            test_cmd: "echo test".to_string(),
            test_pwd: PathBuf::from("."),
            init_cmd: None,
            reset_cmd: None,
            find_number: 1,
            find_number_per_file: 1,
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
            test_cmd: "echo test".to_string(),
            test_pwd: PathBuf::from("."),
            init_cmd: None,
            reset_cmd: None,
            find_number: 1,
            find_number_per_file: 1,
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
            test_cmd: "echo test".to_string(),
            test_pwd: PathBuf::from("."),
            init_cmd: None,
            reset_cmd: None,
            find_number: 1,
            find_number_per_file: 1,
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
        Arc::get_mut(&mut session.base).unwrap().add_mutator(
            LanguageId::Javascript,
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
            Vec::new(),
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
        session.base = Arc::new(
            crate::base::Base::new(
                dir.path().to_path_buf(),
                crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
            )
            .unwrap(),
        );
        Arc::get_mut(&mut session.base).unwrap().add_mutator(
            LanguageId::Javascript,
            crate::twig::TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
            Vec::new(),
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

    #[test]
    fn run_test_in_base_executes_test_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "echo hello".to_string();
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_test(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "hello");
    }

    #[test]
    fn run_test_in_base_uses_pwd() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("src");
        std::fs::create_dir_all(&sub).unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "pwd".to_string();
        config.test_pwd = PathBuf::from("src");
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_test(&config, None).unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        assert!(
            out.trim().ends_with("src"),
            "pwd should end with src, got: {out}"
        );
    }

    #[test]
    fn run_test_in_base_errors_on_absolute_pwd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_pwd = PathBuf::from("/absolute/path");
        let session = Session::new(config.clone()).unwrap();
        let result = session.base().run_test(&config, None);
        assert!(matches!(result, Err(crate::phase::Error::AbsolutePwd(_))));
    }

    #[test]
    fn run_test_in_base_nonzero_exit_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "false".to_string();
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_test(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 1);
    }

    #[test]
    fn run_init_in_base_executes_init_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.init_cmd = Some("echo init".to_string());
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_init(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "init");
    }

    #[test]
    fn run_init_in_base_errors_when_no_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        let session = Session::new(config.clone()).unwrap();
        let result = session.base().run_init(&config, None);
        assert!(matches!(result, Err(crate::phase::Error::NoCmdConfigured)));
    }

    #[test]
    fn run_reset_in_base_executes_reset_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.reset_cmd = Some("echo reset".to_string());
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_reset(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "reset");
    }

    #[test]
    fn run_reset_in_base_errors_when_no_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        let session = Session::new(config.clone()).unwrap();
        let result = session.base().run_reset(&config, None);
        assert!(matches!(result, Err(crate::phase::Error::NoCmdConfigured)));
    }

    #[test]
    fn run_test_in_workspace_executes_in_workspace_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "pwd".to_string();
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_dirty_workspace(&ids[0]);
        let outcome = workspace.run_test(&config, None).unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        assert!(
            out.trim().contains(ids[0].as_str()),
            "should run in workspace dir containing workspace id, got: {out}"
        );
    }

    #[test]
    fn run_test_in_workspace_uses_pwd_relative_to_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "pwd".to_string();
        config.test_pwd = PathBuf::from("src");
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_dirty_workspace(&ids[0]);
        let outcome = workspace.run_test(&config, None).unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        assert!(
            out.trim().ends_with("src"),
            "pwd should end with src, got: {out}"
        );
        assert!(
            out.trim().contains(ids[0].as_str()),
            "should be inside workspace dir, got: {out}"
        );
    }

    #[test]
    fn run_init_in_workspace_executes_init_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.init_cmd = Some("echo workspace_init".to_string());
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_workspace(&ids[0]).unwrap();
        let outcome = workspace.run_init(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(
            String::from_utf8_lossy(outcome.stdout()).trim(),
            "workspace_init"
        );
    }

    #[test]
    fn run_init_in_workspace_errors_when_no_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_workspace(&ids[0]).unwrap();
        let result = workspace.run_init(&config, None);
        assert!(matches!(result, Err(crate::phase::Error::NoCmdConfigured)));
    }

    #[test]
    fn run_reset_in_workspace_executes_reset_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.reset_cmd = Some("echo workspace_reset".to_string());
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_workspace(&ids[0]).unwrap();
        let outcome = workspace.run_reset(&config, None).unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(
            String::from_utf8_lossy(outcome.stdout()).trim(),
            "workspace_reset"
        );
    }

    #[test]
    fn run_reset_in_workspace_errors_when_no_cmd() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "const x = 1;").unwrap();
        let mut session = Session::new(config.clone()).unwrap();
        let ids = session.tend_workspaces(1).unwrap();
        let workspace = session.bind_workspace(&ids[0]).unwrap();
        let result = workspace.run_reset(&config, None);
        assert!(matches!(result, Err(crate::phase::Error::NoCmdConfigured)));
    }

    #[test]
    fn run_test_in_base_splits_cmd_at_whitespace() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "echo one two three".to_string();
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_test(&config, None).unwrap();
        assert_eq!(
            String::from_utf8_lossy(outcome.stdout()).trim(),
            "one two three"
        );
    }

    #[test]
    fn run_test_in_base_default_pwd_is_root() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        config.test_cmd = "pwd".to_string();
        let session = Session::new(config.clone()).unwrap();
        let outcome = session.base().run_test(&config, None).unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        let actual = PathBuf::from(out.trim()).canonicalize().unwrap();
        let expected = dir.path().canonicalize().unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn set_state_records_outcome_on_mutation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        let hash = mutation.hash().unwrap();
        session
            .set_state(&mutation, crate::state::Status::Missed)
            .unwrap();

        let state = session.get_state().get(&hash).unwrap();
        assert!(state.has_outcome());
        assert!(state.outcome_at().is_some());
    }

    #[test]
    fn set_state_persists_to_disk() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        let hash = mutation.hash().unwrap();
        session
            .set_state(&mutation, crate::state::Status::Caught)
            .unwrap();

        let config2 = make_js_config(dir.path());
        let session2 = Session::new(config2).unwrap();
        let persisted = session2.get_state().get(&hash);
        assert!(persisted.is_some());
        assert!(persisted.unwrap().has_outcome());
    }

    #[test]
    fn set_state_sets_at_automatically() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let before = chrono::Utc::now() - chrono::Duration::seconds(1);
        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        let hash = mutation.hash().unwrap();
        session
            .set_state(&mutation, crate::state::Status::Missed)
            .unwrap();
        let after = chrono::Utc::now() + chrono::Duration::seconds(1);

        let state = session.get_state().get(&hash).unwrap();
        let at = state.outcome_at().unwrap();
        assert!(
            at >= before && at <= after,
            "expected {before} <= {at} <= {after}"
        );
    }

    #[test]
    fn set_state_overwrites_previous_outcome() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        let hash = mutation.hash().unwrap();
        session
            .set_state(&mutation, crate::state::Status::Missed)
            .unwrap();
        let first_at = session
            .get_state()
            .get(&hash)
            .unwrap()
            .outcome_at()
            .unwrap();

        std::thread::sleep(std::time::Duration::from_secs(1));
        session
            .set_state(&mutation, crate::state::Status::Caught)
            .unwrap();
        let second_at = session
            .get_state()
            .get(&hash)
            .unwrap()
            .outcome_at()
            .unwrap();

        assert!(second_at > first_at);
    }

    #[test]
    fn set_state_preserves_mutation() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let config = make_js_config(dir.path());
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        let hash = mutation.hash().unwrap();
        session
            .set_state(&mutation, crate::state::Status::Caught)
            .unwrap();

        let state = session.get_state().get(&hash).unwrap();
        assert!(state.has_outcome());
        assert_eq!(state.mutation(), &mutation);
    }

    #[test]
    fn find_best_mutations_empty_when_no_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        let session = Session::new(config).unwrap();
        let results = session.find_best_mutations().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn find_best_mutations_returns_mutations_needing_test() {
        let dir = tempfile::tempdir().unwrap();
        let config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();
        let total = session.get_count_mutation_needing_test();
        assert!(total > 0);

        let results = session.find_best_mutations().unwrap();
        assert_eq!(results.len(), 1);
        let (hash, state, score) = &results[0];
        assert!(state.mutation().mutant().twig().path().ends_with("a.js"));
        assert!(*score >= 0.0 && *score <= 1.0);
        assert!(session.get_state().get(hash).is_some());
    }

    #[test]
    fn find_best_mutations_respects_number_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return a + b; }").unwrap();
        config.find_number = 2;
        config.find_number_per_file = 10;
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();
        let total = session.get_count_mutation_needing_test();
        assert!(total > 2, "need more than 2 mutations to test limit, got {total}");

        let results = session.find_best_mutations().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn find_best_mutations_respects_number_per_file_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return a + b; }").unwrap();
        std::fs::write(dir.path().join("src/b.js"), "function bar() { return c - d; }").unwrap();
        config.find_number = 100;
        config.find_number_per_file = 1;
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let results = session.find_best_mutations().unwrap();
        let a_count = results.iter().filter(|(_, s, _)| {
            s.mutation().mutant().twig().path().ends_with("a.js")
        }).count();
        let b_count = results.iter().filter(|(_, s, _)| {
            s.mutation().mutant().twig().path().ends_with("b.js")
        }).count();
        assert_eq!(a_count, 1);
        assert_eq!(b_count, 1);
    }

    #[test]
    fn find_best_mutations_excludes_caught() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "function foo() { return 1; }").unwrap();
        config.find_number = 100;
        config.find_number_per_file = 100;
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();
        let total_before = session.get_count_mutation_needing_test();

        let mutation: Mutation = session.base().mutations().next().unwrap().unwrap();
        session.set_state(&mutation, crate::state::Status::Caught).unwrap();
        session.mutations_needing_test = Session::<MinimalConfig>::derive_mutations_needing_testing(&session.mutations_state);

        let results = session.find_best_mutations().unwrap();
        assert_eq!(results.len(), total_before - 1);
    }

    #[test]
    fn find_best_mutations_sorted_by_score_descending() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "if (x) { return a + b; }").unwrap();
        config.find_number = 100;
        config.find_number_per_file = 100;
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let results = session.find_best_mutations().unwrap();
        assert!(results.len() > 1);
        for w in results.windows(2) {
            assert!(w[0].2 >= w[1].2, "results should be sorted descending by score: {} >= {}", w[0].2, w[1].2);
        }
    }

    #[test]
    fn find_best_mutations_scores_between_zero_and_one() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_js_config(dir.path());
        std::fs::write(dir.path().join("src/a.js"), "if (x) { return a + b; }").unwrap();
        config.find_number = 100;
        config.find_number_per_file = 100;
        let mut session = Session::new(config).unwrap();
        session.tend_add_missing_states().unwrap();

        let results = session.find_best_mutations().unwrap();
        for (_, _, score) in &results {
            assert!(*score >= 0.0 && *score <= 1.0, "score {score} out of range");
        }
    }
}

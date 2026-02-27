use crate::Outcome;
use crate::languages::LanguageId;
use serde::{Deserialize, Serialize};
use serde_value::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn default_parallelism() -> u32 {
    1
}
fn default_working() -> String {
    "/tmp/bough/work".into()
}
fn default_state() -> String {
    "./bough/".into()
}
fn default_logs() -> String {
    "/tmp/bough/logs".into()
}
fn default_pwd() -> PathBuf {
    PathBuf::from(".")
}
fn default_test_phase() -> Phase {
    Phase {
        commands: vec!["exit 1".into()],
        ..Phase::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VcsConfig {
    #[default]
    None,
    Git {
        commit: String,
    },
    Jj {
        rev: String,
    },
    Mercurial,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    #[default]
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(skip)]
    _sealed: (),
    active_runner: String,
    vcs: VcsConfig,
    #[serde(default = "default_parallelism")]
    parallelism: u32,
    ordering: Ordering,
    dirs: Dirs,
    #[serde(flatten)]
    runners: HashMap<String, Runner>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            _sealed: (),
            active_runner: String::new(),
            vcs: VcsConfig::default(),
            parallelism: default_parallelism(),
            ordering: Ordering::default(),
            dirs: Dirs::default(),
            runners: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dirs {
    #[serde(default = "default_working")]
    working: String,
    #[serde(default = "default_state")]
    state: String,
    #[serde(default = "default_logs")]
    logs: String,
}

impl Default for Dirs {
    fn default() -> Self {
        Self {
            working: default_working(),
            state: default_state(),
            logs: default_logs(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Runner {
    #[serde(default = "default_pwd")]
    pwd: PathBuf,
    treat_timeouts_as: Outcome,
    init: Option<Phase>,
    get_test_ids: Option<String>,
    reset: Option<Phase>,
    #[serde(default = "default_test_phase")]
    test: Phase,
    test_ids: Option<TestIds>,
    #[serde(flatten)]
    mutate: HashMap<LanguageId, MutateLanguage>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            pwd: PathBuf::from("."),
            treat_timeouts_as: Outcome::Caught,
            init: None,
            reset: None,
            test: default_test_phase(),
            test_ids: None,
            mutate: HashMap::new(),
            get_test_ids: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    pwd: Option<PathBuf>,
    timeout: Timeout,
    env: HashMap<String, String>,
    commands: Vec<String>,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            pwd: None,
            timeout: Timeout::default(),
            env: HashMap::new(),
            commands: Vec::new(),
        }
    }
}

impl Phase {
    pub fn commands(&self) -> &[String] {
        &self.commands
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeout {
    absolute: Option<u64>,
    relative: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestIds {
    get_all: String,
    get_failed: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateLanguage {
    files: FileFilter,
    mutants: MutantFilter,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileFilter {
    include: Vec<String>,
    exclude: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutantFilter {
    skip: Vec<MutantSkip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MutantSkip {
    Query { query: String },
    Kind { kind: HashMap<String, String> },
}

fn deep_merge(base: Value, patch: Value) -> Value {
    match (base, patch) {
        (Value::Map(mut base_map), Value::Map(patch_map)) => {
            for (k, v) in patch_map {
                let merged = match base_map.remove(&k) {
                    Some(existing) => deep_merge(existing, v),
                    None => v,
                };
                base_map.insert(k, merged);
            }
            Value::Map(base_map)
        }
        (_, patch) => patch,
    }
}

pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn from_value(value: Value) -> Self {
        let config = Config::deserialize(value).expect("failed to deserialize config");
        Self { config }
    }

    pub fn override_with(mut self, patch: Value) -> Self {
        let base = serde_value::to_value(self.config).expect("failed to serialize config");
        let merged = deep_merge(base, patch);
        self.config = Config::deserialize(merged).expect("failed to deserialize merged config");
        self
    }

    pub fn build(mut self) -> Result<Config, ConfigError> {
        self.config.resolve_paths();
        self.check_invariants()?;
        Ok(self.config)
    }

    fn check_invariants(&self) -> Result<(), ConfigError> {
        let runner = &self.config.active_runner;
        if !runner.is_empty() && !self.config.runners.contains_key(runner) {
            return Err(ConfigError::UnknownRunner {
                name: runner.clone(),
                available: self.config.runners.keys().cloned().collect(),
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    UnknownRunner {
        name: String,
        available: Vec<String>,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::UnknownRunner { name, available } => {
                write!(
                    f,
                    "runner '{}' not found in config (available: {})",
                    name,
                    if available.is_empty() {
                        "none".to_string()
                    } else {
                        available.join(", ")
                    }
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn active_runner(&self) -> &str {
        &self.active_runner
    }

    pub fn vcs(&self) -> &VcsConfig {
        &self.vcs
    }

    pub fn parallelism(&self) -> u32 {
        self.parallelism
    }

    pub fn ordering(&self) -> Ordering {
        self.ordering
    }

    pub fn working_dir(&self) -> &str {
        &self.dirs.working
    }

    pub fn state_dir(&self) -> &str {
        &self.dirs.state
    }

    pub fn logs_dir(&self) -> &str {
        &self.dirs.logs
    }

    pub fn runner_names(&self) -> Vec<String> {
        self.runners.keys().cloned().collect()
    }

    pub fn resolved_runner_name(&self) -> Option<&str> {
        if self.active_runner.is_empty() {
            self.runners.keys().next().map(|s| s.as_str())
        } else if self.runners.contains_key(&self.active_runner) {
            Some(&self.active_runner)
        } else {
            None
        }
    }

    pub fn runner(&self, runner: &str) -> Option<&Runner> {
        self.runners.get(runner)
    }

    pub fn runner_pwd(&self, runner: &str) -> Option<&Path> {
        self.runners.get(runner).map(|r| r.pwd.as_path())
    }

    pub fn resolve_pwd<'a>(
        &self,
        runner: Option<&'a Runner>,
        phase: Option<&'a Phase>,
    ) -> &'a Path {
        static DEFAULT: &str = ".";
        if let Some(pwd) = phase.and_then(|p| p.pwd.as_deref()) {
            return pwd;
        }
        if let Some(r) = runner {
            return &r.pwd;
        }
        Path::new(DEFAULT)
    }

    pub fn resolve_timeout_absolute(
        &self,
        _runner: Option<&Runner>,
        phase: Option<&Phase>,
    ) -> Option<u64> {
        phase.and_then(|p| p.timeout.absolute)
    }

    pub fn resolve_timeout_relative(
        &self,
        _runner: Option<&Runner>,
        phase: Option<&Phase>,
    ) -> Option<u64> {
        phase.and_then(|p| p.timeout.relative)
    }

    pub fn resolve_env<'a>(
        &self,
        _runner: Option<&'a Runner>,
        phase: Option<&'a Phase>,
    ) -> &'a HashMap<String, String> {
        static EMPTY: std::sync::LazyLock<HashMap<String, String>> =
            std::sync::LazyLock::new(HashMap::new);
        phase.map(|p| &p.env).unwrap_or(&EMPTY)
    }

    pub fn runner_treat_timeouts_as(&self, runner: &str) -> Option<Outcome> {
        self.runners.get(runner).map(|r| r.treat_timeouts_as)
    }

    pub fn runner_init_phase(&self, runner: &str) -> Option<&Phase> {
        self.runners.get(runner).and_then(|r| r.init.as_ref())
    }

    pub fn runner_reset_phase(&self, runner: &str) -> Option<&Phase> {
        self.runners.get(runner).and_then(|r| r.reset.as_ref())
    }

    pub fn runner_test_phase(&self, runner: &str) -> Option<&Phase> {
        self.runners.get(runner).map(|r| &r.test)
    }

    pub fn runner_test_ids_get_all(&self, runner: &str) -> Option<&str> {
        self.runners
            .get(runner)
            .and_then(|r| r.test_ids.as_ref())
            .map(|t| t.get_all.as_str())
    }

    pub fn runner_test_ids_get_failed(&self, runner: &str) -> Option<&str> {
        self.runners
            .get(runner)
            .and_then(|r| r.test_ids.as_ref())
            .map(|t| t.get_failed.as_str())
    }

    pub fn mutate_languages(&self, runner: &str) -> Vec<LanguageId> {
        self.runners
            .get(runner)
            .map(|r| r.mutate.keys().copied().collect())
            .unwrap_or_default()
    }

    pub fn file_includes(&self, runner: &str, lang: LanguageId) -> Vec<String> {
        self.runners
            .get(runner)
            .and_then(|r| r.mutate.get(&lang))
            .map(|m| m.files.include.clone())
            .unwrap_or_default()
    }

    pub fn file_excludes(&self, runner: &str, lang: LanguageId) -> Vec<String> {
        self.runners
            .get(runner)
            .and_then(|r| r.mutate.get(&lang))
            .map(|m| m.files.exclude.clone())
            .unwrap_or_default()
    }

    pub fn mutant_skips(&self, runner: &str, lang: LanguageId) -> Vec<MutantSkip> {
        self.runners
            .get(runner)
            .and_then(|r| r.mutate.get(&lang))
            .map(|m| m.mutants.skip.clone())
            .unwrap_or_default()
    }

    fn resolve_paths(&mut self) {
        let cwd = std::env::current_dir().expect("failed to get current directory");
        let resolve = |p: &mut String| {
            let path = std::path::PathBuf::from(&*p);
            if !path.is_absolute() {
                *p = cwd
                    .join(path)
                    .canonicalize()
                    .unwrap_or_else(|_| cwd.join(&*p))
                    .to_string_lossy()
                    .into_owned();
            }
        };

        resolve(&mut self.dirs.working);
        resolve(&mut self.dirs.state);
        resolve(&mut self.dirs.logs);

        for _runner in self.runners.values_mut() {
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_ideal_config() {
        let toml_str = include_str!("ideal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse ideal config");

        assert_eq!(
            config.vcs,
            VcsConfig::Jj {
                rev: "trunk()".into()
            }
        );
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);

        assert_eq!(config.dirs.working, "/tmp/bough/work");
        assert_eq!(config.dirs.state, "/tmp/bough/state");
        assert_eq!(config.dirs.logs, "/tmp/bough/logs");

        let vitest = &config.runners["vitest"];
        assert_eq!(vitest.treat_timeouts_as, Outcome::Missed);

        let init = vitest.init.as_ref().unwrap();
        assert_eq!(init.pwd.as_deref(), Some(Path::new("./examples/vitest")));
        assert_eq!(init.commands, vec!["npm install"]);

        assert_eq!(vitest.test.timeout.absolute, Some(30));
        assert_eq!(vitest.test.timeout.relative, Some(3));
        assert_eq!(vitest.test.env["NODE_ENV"], "production");
        assert_eq!(vitest.test.commands, vec!["npx run build", "npx run test"]);

        let js_mutate = &vitest.mutate[&LanguageId::Javascript];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);

        let cargo = &config.runners["cargo"];
        assert_eq!(cargo.pwd, PathBuf::from("./examples/cargo"));
        assert_eq!(cargo.test.commands, vec!["cargo test"]);
    }

    #[test]
    fn deserialize_minimal_config() {
        let toml_str = include_str!("minimal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse minimal config");

        assert_eq!(config.vcs, VcsConfig::None);
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);
        assert_eq!(config.dirs, Dirs::default());

        let vitest = &config.runners["vitest"];
        assert_eq!(vitest.treat_timeouts_as, Outcome::Caught);
        assert!(vitest.init.is_none());
        assert!(vitest.reset.is_none());

        assert_eq!(vitest.test.commands, vec!["npx vitest run"]);
        assert_eq!(vitest.test.pwd, None);
        assert!(vitest.test.env.is_empty());
        assert_eq!(vitest.test.timeout, Timeout::default());

        let ts_mutate = &vitest.mutate[&LanguageId::Typescript];
        assert_eq!(ts_mutate.files.include, vec!["src/**/*.ts"]);
        assert!(ts_mutate.files.exclude.is_empty());
        assert!(ts_mutate.mutants.skip.is_empty());
    }

    fn toml_to_value(s: &str) -> Value {
        let tv: toml::Value = toml::from_str(s).unwrap();
        serde_value::to_value(tv).unwrap()
    }

    fn build_with_override(base: &str, patch: &str) -> Config {
        ConfigBuilder::from_value(toml_to_value(base))
            .override_with(toml_to_value(patch))
            .build()
            .unwrap()
    }

    #[test]
    fn override_scalar() {
        let config = build_with_override("", "parallelism = 4");
        assert_eq!(config.parallelism, 4);
    }

    #[test]
    fn override_nested_preserves_siblings() {
        let config = build_with_override(
            include_str!("ideal.config.toml"),
            r#"
            [dirs]
            working = "/override"
            "#,
        );
        assert_eq!(config.dirs.working, "/override");
        assert_eq!(config.dirs.logs, "/tmp/bough/logs");
    }

    #[test]
    fn override_vec_replaces() {
        let config = build_with_override(
            include_str!("minimal.config.toml"),
            r#"
            [vitest.test]
            commands = ["npm test"]
            "#,
        );
        assert_eq!(config.runners["vitest"].test.commands, vec!["npm test"]);
    }

    #[test]
    fn override_map_adds() {
        let config = build_with_override(
            include_str!("minimal.config.toml"),
            r#"
            [vitest.test.env]
            FOO = "bar"
            "#,
        );
        assert_eq!(config.runners["vitest"].test.env["FOO"], "bar");
    }

    #[test]
    fn override_deep_merge_runner() {
        let config = build_with_override(
            include_str!("ideal.config.toml"),
            r#"
            [vitest]
            treat_timeouts_as = "Caught"
            "#,
        );
        assert_eq!(config.runners["vitest"].treat_timeouts_as, Outcome::Caught);
        assert!(config.runners["vitest"].init.is_some());
    }

    #[test]
    fn runner_without_test_defaults_to_exit_1() {
        let config: Config = toml::from_str(
            r#"
            [myrunner]
            pwd = "."
        "#,
        )
        .unwrap();
        assert_eq!(config.runners["myrunner"].test.commands, vec!["exit 1"]);
    }

    #[test]
    fn defaults_are_sane() {
        let config: Config = toml::from_str("").expect("empty config should parse");
        assert_eq!(config.vcs, VcsConfig::None);
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);
        assert_eq!(config.dirs, Dirs::default());
        assert!(config.runners.is_empty());
    }

    fn make_runner(pwd: &str) -> Runner {
        Runner {
            pwd: PathBuf::from(pwd),
            ..Runner::default()
        }
    }

    fn make_phase(pwd: Option<&str>) -> Phase {
        Phase {
            pwd: pwd.map(PathBuf::from),
            ..Phase::default()
        }
    }

    mod resolve_pwd {
        use super::*;

        #[test]
        fn phase_pwd_wins() {
            let config: Config = toml::from_str("").unwrap();
            let runner = make_runner("./runner");
            let phase = make_phase(Some("./phase"));
            assert_eq!(
                config.resolve_pwd(Some(&runner), Some(&phase)),
                Path::new("./phase")
            );
        }

        #[test]
        fn falls_back_to_runner_pwd() {
            let config: Config = toml::from_str("").unwrap();
            let runner = make_runner("./runner");
            let phase = make_phase(None);
            assert_eq!(
                config.resolve_pwd(Some(&runner), Some(&phase)),
                Path::new("./runner")
            );
        }

        #[test]
        fn falls_back_to_dot_without_runner() {
            let config: Config = toml::from_str("").unwrap();
            let phase = make_phase(None);
            assert_eq!(config.resolve_pwd(None, Some(&phase)), Path::new("."));
        }

        #[test]
        fn falls_back_to_dot_with_no_args() {
            let config: Config = toml::from_str("").unwrap();
            assert_eq!(config.resolve_pwd(None, None), Path::new("."));
        }

        #[test]
        fn runner_pwd_without_phase() {
            let config: Config = toml::from_str("").unwrap();
            let runner = make_runner("./sub");
            assert_eq!(config.resolve_pwd(Some(&runner), None), Path::new("./sub"));
        }
    }

    mod resolve_timeout {
        use super::*;

        #[test]
        fn returns_phase_timeout() {
            let config: Config = toml::from_str("").unwrap();
            let phase = Phase {
                timeout: Timeout {
                    absolute: Some(30),
                    relative: Some(3),
                },
                ..Phase::default()
            };
            assert_eq!(
                config.resolve_timeout_absolute(None, Some(&phase)),
                Some(30)
            );
            assert_eq!(
                config.resolve_timeout_relative(None, Some(&phase)),
                Some(3)
            );
        }

        #[test]
        fn returns_none_without_phase() {
            let config: Config = toml::from_str("").unwrap();
            assert_eq!(config.resolve_timeout_absolute(None, None), None);
            assert_eq!(config.resolve_timeout_relative(None, None), None);
        }

        #[test]
        fn returns_none_for_default_phase() {
            let config: Config = toml::from_str("").unwrap();
            let phase = Phase::default();
            assert_eq!(
                config.resolve_timeout_absolute(None, Some(&phase)),
                None
            );
        }
    }

    mod resolve_env {
        use super::*;

        #[test]
        fn returns_phase_env() {
            let config: Config = toml::from_str("").unwrap();
            let phase = Phase {
                env: HashMap::from([("K".into(), "V".into())]),
                ..Phase::default()
            };
            let env = config.resolve_env(None, Some(&phase));
            assert_eq!(env["K"], "V");
        }

        #[test]
        fn returns_empty_without_phase() {
            let config: Config = toml::from_str("").unwrap();
            let env = config.resolve_env(None, None);
            assert!(env.is_empty());
        }

        #[test]
        fn returns_empty_for_default_phase() {
            let config: Config = toml::from_str("").unwrap();
            let phase = Phase::default();
            let env = config.resolve_env(None, Some(&phase));
            assert!(env.is_empty());
        }
    }
}

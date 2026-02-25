use crate::Outcome;
use crate::io::Render;
use serde::{Deserialize, Serialize};
use serde_value::Value;
use std::collections::HashMap;

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
fn default_pwd() -> String {
    ".".into()
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
    pub active_runner: String,
    pub vcs: VcsConfig,
    #[serde(default = "default_parallelism")]
    pub parallelism: u32,
    pub ordering: Ordering,
    pub dirs: Dirs,
    #[serde(flatten)]
    pub runners: HashMap<String, Runner>,
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
    pub working: String,
    #[serde(default = "default_state")]
    pub state: String,
    #[serde(default = "default_logs")]
    pub logs: String,
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
    pub pwd: String,
    pub treat_timeouts_as: Outcome,
    pub init: Option<Phase>,
    pub reset: Option<Phase>,
    #[serde(default = "default_test_phase")]
    pub test: Phase,
    pub mutate: HashMap<String, MutateLanguage>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            pwd: default_pwd(),
            treat_timeouts_as: Outcome::Caught,
            init: None,
            reset: None,
            test: default_test_phase(),
            mutate: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    #[serde(default = "default_pwd")]
    pub pwd: String,
    pub timeout: Timeout,
    pub env: HashMap<String, String>,
    pub commands: Vec<String>,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            pwd: default_pwd(),
            timeout: Timeout::default(),
            env: HashMap::new(),
            commands: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeout {
    pub absolute: Option<u64>,
    pub relative: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateLanguage {
    pub files: FileFilter,
    pub mutants: MutantFilter,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutantFilter {
    pub skip: Vec<MutantSkip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MutantSkip {
    Lisp { lisp: String },
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

        for runner in self.runners.values_mut() {
            resolve(&mut runner.pwd);
            if let Some(phase) = &mut runner.init {
                resolve(&mut phase.pwd);
            }
            if let Some(phase) = &mut runner.reset {
                resolve(&mut phase.pwd);
            }
            resolve(&mut runner.test.pwd);
        }
    }
}

impl Render for Config {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize config")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        format!("{self:#?}")
    }

    fn render_markdown(&self, _depth: u8) -> String {
        format!("```\n{self:#?}\n```")
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
        assert_eq!(init.pwd, "./examples/vitest");
        assert_eq!(init.commands, vec!["npm install"]);

        assert_eq!(vitest.test.timeout.absolute, Some(30));
        assert_eq!(vitest.test.timeout.relative, Some(3));
        assert_eq!(vitest.test.env["NODE_ENV"], "production");
        assert_eq!(vitest.test.commands, vec!["npx run build", "npx run test"]);

        let js_mutate = &vitest.mutate["js"];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);

        let cargo = &config.runners["cargo"];
        assert_eq!(cargo.pwd, "./examples/cargo");
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
        assert_eq!(vitest.treat_timeouts_as, Outcome::Missed);
        assert!(vitest.init.is_none());
        assert!(vitest.reset.is_none());

        assert_eq!(vitest.test.commands, vec!["npx vitest run"]);
        assert_eq!(vitest.test.pwd, ".");
        assert!(vitest.test.env.is_empty());
        assert_eq!(vitest.test.timeout, Timeout::default());

        let ts_mutate = &vitest.mutate["ts"];
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
}

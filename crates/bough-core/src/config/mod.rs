pub mod phase;
pub mod suite;

pub use phase::{PhaseConfig, PhaseMetaConfig, TimeoutConfig};
pub use suite::{
    FileSourceConfig, MutantFilterConfig, MutantSkipConfig, MutateLanguageConfig, OrderingConfig,
    SuiteConfig, TestIdsConfig,
};

use crate::phase::{Phase, PhaseRunner, RunIn};
use crate::suite::Suite;
use serde::{Deserialize, Serialize};
use serde_value::Value;
use std::collections::HashMap;
use std::path::PathBuf;

fn default_threads() -> u32 {
    1
}
fn default_bough_dir() -> PathBuf {
    PathBuf::from("./.bough")
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SuiteId(pub(crate) String);

impl std::fmt::Display for SuiteId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl SuiteId {
    pub fn try_parse(config: &Config, name: &str) -> Result<Self, ConfigError> {
        let id = Self(name.to_string());
        if config.suites.contains_key(&id) {
            Ok(id)
        } else {
            Err(ConfigError::UnknownSuite {
                name: id,
                available: config.suites.keys().cloned().collect(),
            })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub(crate) active_suite: SuiteId,
    #[serde(default = "default_threads")]
    pub(crate) threads: u32,
    #[serde(default = "default_bough_dir")]
    pub(crate) bough_dir: PathBuf,
    #[serde(skip)]
    pub(crate) source_dir: PathBuf,
    #[serde(flatten)]
    pub(crate) suites: HashMap<SuiteId, SuiteConfig>,
}

impl Config {
    pub fn new_suite<'a>(&'a self, id: &SuiteId) -> Result<Suite<'a>, ConfigError> {
        let suite = self.suite_or_err(id)?;
        Ok(Suite::new(suite))
    }

    pub fn new_phase_runner(
        &self,
        suite_id: &SuiteId,
        phase: Phase,
        run_in: RunIn,
    ) -> Result<PhaseRunner, ConfigError> {
        let suite = self.suite_or_err(suite_id)?;
        let phase_config = suite.get_phase_config(phase).ok_or(ConfigError::PhaseNotConfigured {
            phase,
            suite: suite_id.clone(),
        })?;

        let base = match &run_in {
            RunIn::SourceDir => self.source_dir.clone(),
            RunIn::Workspace(id) => self.resolve_path(&self.bough_dir).join("workspaces").join(id),
        };

        let pwd = phase_config.meta.pwd.as_deref()
            .or_else(|| suite.meta_defaults.pwd.as_deref())
            .map(|p| base.join(p))
            .unwrap_or(base);

        let mut env = suite.meta_defaults.env.clone();
        env.extend(phase_config.meta.env.clone());

        let timeout = TimeoutConfig {
            absolute: phase_config.meta.timeout.absolute.or(suite.meta_defaults.timeout.absolute),
            relative: phase_config.meta.timeout.relative.or(suite.meta_defaults.timeout.relative),
        };

        Ok(PhaseRunner {
            pwd,
            env,
            timeout,
            commands: phase_config.commands.clone(),
        })
    }

    fn resolve_path(&self, path: &std::path::Path) -> PathBuf {
        if path.is_absolute() {
            path.to_owned()
        } else {
            self.source_dir.join(path)
        }
    }

    fn suite_or_err(&self, id: &SuiteId) -> Result<&SuiteConfig, ConfigError> {
        self.suites
            .get(id)
            .ok_or_else(|| ConfigError::UnknownSuite {
                name: id.clone(),
                available: self.suites.keys().cloned().collect(),
            })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            active_suite: SuiteId::default(),
            threads: default_threads(),
            bough_dir: default_bough_dir(),
            source_dir: PathBuf::default(),
            suites: HashMap::new(),
        }
    }
}

pub struct ConfigBuilder {
    value: Value,
    source_dir: PathBuf,
}

impl ConfigBuilder {
    pub fn new(source_dir: PathBuf) -> Self {
        Self {
            value: Value::Map(Default::default()),
            source_dir,
        }
    }

    pub fn from_value(self, value: Value) -> Self {
        Self { value, ..self }
    }

    pub fn override_with(mut self, patch: Value) -> Self {
        self.value = Self::deep_merge(self.value, patch);
        self
    }

    fn deep_merge(base: Value, patch: Value) -> Value {
        match (base, patch) {
            (Value::Map(mut base_map), Value::Map(patch_map)) => {
                for (k, v) in patch_map {
                    let merged = match base_map.remove(&k) {
                        Some(existing) => Self::deep_merge(existing, v),
                        None => v,
                    };
                    base_map.insert(k, merged);
                }
                Value::Map(base_map)
            }
            (_, patch) => patch,
        }
    }

    pub fn build(self) -> Result<Config, ConfigError> {
        let mut config = Config::deserialize(self.value).map_err(ConfigError::Deserialize)?;
        config.source_dir = self.source_dir;
        Self::check_invariants(&config)?;
        Ok(config)
    }

    fn check_invariants(config: &Config) -> Result<(), ConfigError> {
        let suite = &config.active_suite;
        if !suite.0.is_empty() && !config.suites.contains_key(suite) {
            return Err(ConfigError::UnknownSuite {
                name: suite.clone(),
                available: config.suites.keys().cloned().collect(),
            });
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ConfigError {
    UnknownSuite {
        name: SuiteId,
        available: Vec<SuiteId>,
    },
    PhaseNotConfigured {
        phase: Phase,
        suite: SuiteId,
    },
    Deserialize(serde_value::DeserializerError),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::UnknownSuite { name, available } => {
                write!(
                    f,
                    "suite '{}' not found in config (available: {})",
                    name,
                    if available.is_empty() {
                        "none".to_string()
                    } else {
                        available
                            .iter()
                            .map(|s| s.0.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                )
            }
            ConfigError::PhaseNotConfigured { phase, suite } => {
                write!(f, "phase '{phase:?}' not configured in suite '{suite}'")
            }
            ConfigError::Deserialize(e) => write!(f, "failed to deserialize config: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::LanguageId;
    use crate::phase::Phase;

    use crate::Outcome;

    #[test]
    fn deserialize_ideal_config() {
        let toml_str = include_str!("ideal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse ideal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));

        let vitest = &config.suites[&SuiteId("vitest".into())];
        assert_eq!(vitest.ordering, OrderingConfig::Alphabetical);
        assert_eq!(vitest.treat_timeouts_as, Outcome::Missed);

        let init = &vitest.phases[&Phase::Init];
        assert_eq!(init.meta.pwd, Some(PathBuf::from("./examples/vitest")));
        assert_eq!(init.commands, vec!["npm install"]);

        let test = &vitest.phases[&Phase::Test];
        assert_eq!(test.meta.timeout.absolute, Some(30));
        assert_eq!(test.meta.timeout.relative, Some(3));
        assert_eq!(test.meta.env["NODE_ENV"], "production");
        assert_eq!(test.commands, vec!["npx run build", "npx run test"]);

        let js_mutate = &vitest.mutate[&LanguageId::Javascript];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);

        let cargo = &config.suites[&SuiteId("cargo".into())];
        assert_eq!(
            cargo.meta_defaults.pwd,
            Some(PathBuf::from("./examples/cargo"))
        );
        assert_eq!(cargo.phases[&Phase::Test].commands, vec!["cargo test"]);
    }

    #[test]
    fn deserialize_minimal_config() {
        let toml_str = include_str!("minimal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse minimal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));

        let vitest = &config.suites[&SuiteId("vitest".into())];
        assert_eq!(vitest.ordering, OrderingConfig::Random);
        assert_eq!(vitest.treat_timeouts_as, Outcome::Caught);
        assert!(!vitest.phases.contains_key(&Phase::Init));
        assert!(!vitest.phases.contains_key(&Phase::Reset));

        let test = &vitest.phases[&Phase::Test];
        assert_eq!(test.commands, vec!["npx vitest run"]);
        assert_eq!(test.meta.pwd, None);
        assert!(test.meta.env.is_empty());
        assert_eq!(test.meta.timeout, TimeoutConfig::default());

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
        ConfigBuilder::new(PathBuf::new())
            .from_value(toml_to_value(base))
            .override_with(toml_to_value(patch))
            .build()
            .unwrap()
    }

    #[test]
    fn override_scalar() {
        let config = build_with_override("", "threads = 4");
        assert_eq!(config.threads, 4);
    }

    #[test]
    fn override_bough_dir() {
        let config = build_with_override(
            include_str!("ideal.config.toml"),
            r#"bough_dir = "/override""#,
        );
        assert_eq!(config.bough_dir, PathBuf::from("/override"));
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
        assert_eq!(
            config.suites[&SuiteId("vitest".into())].phases[&Phase::Test].commands,
            vec!["npm test"]
        );
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
        assert_eq!(
            config.suites[&SuiteId("vitest".into())].phases[&Phase::Test]
                .meta
                .env["FOO"],
            "bar"
        );
    }

    #[test]
    fn override_deep_merge_suite() {
        let config = build_with_override(
            include_str!("ideal.config.toml"),
            r#"
            [vitest]
            treat_timeouts_as = "Caught"
            "#,
        );
        assert_eq!(
            config.suites[&SuiteId("vitest".into())].treat_timeouts_as,
            Outcome::Caught
        );
        assert!(
            config.suites[&SuiteId("vitest".into())]
                .phases
                .contains_key(&Phase::Init)
        );
    }

    #[test]
    fn defaults_are_sane() {
        let config: Config = toml::from_str("").expect("empty config should parse");
        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert!(config.suites.is_empty());
    }
}

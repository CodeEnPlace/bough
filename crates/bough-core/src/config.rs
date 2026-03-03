use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize, clap::ValueEnum, bough_typed_hash::HashInto)]
#[serde(rename_all = "lowercase")]
pub enum LanguageId {
    #[serde(alias = "js")]
    #[value(alias = "js")]
    Javascript,
    #[serde(alias = "ts")]
    #[value(alias = "ts")]
    Typescript,
}

fn default_threads() -> u32 {
    1
}
fn default_bough_dir() -> PathBuf {
    PathBuf::from("./.bough")
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseMetaConfig {
    pub(crate) pwd: Option<PathBuf>,
    pub(crate) timeout: TimeoutConfig,
    pub(crate) env: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseConfig {
    #[serde(flatten)]
    pub(crate) meta: PhaseMetaConfig,
    pub(crate) command: String,
}

impl PhaseConfig {
    pub fn command(&self) -> &str {
        &self.command
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    pub(crate) absolute: Option<u64>,
    pub(crate) relative: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OrderingConfig {
    #[default]
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestIdsConfig {
    pub(crate) get_all: String,
    pub(crate) get_failed: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateLanguageConfig {
    pub(crate) files: FileSourceConfig,
    pub(crate) mutants: MutantFilterConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileSourceConfig {
    pub(crate) include: Vec<String>,
    pub(crate) exclude: Vec<String>,
    pub(crate) ignore_files: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutantFilterConfig {
    pub(crate) skip: Vec<MutantSkipConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MutantSkipConfig {
    Query { query: String },
    Kind { kind: HashMap<String, String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    #[serde(default = "default_threads")]
    pub(crate) threads: u32,
    #[serde(default = "default_bough_dir")]
    pub(crate) bough_dir: PathBuf,
    #[serde(skip)]
    pub(crate) source_dir: PathBuf,
    pub(crate) ordering: OrderingConfig,
    pub(crate) treat_timeouts_as: Outcome,
    pub(crate) test_ids: Option<TestIdsConfig>,
    #[serde(flatten)]
    pub(crate) meta_defaults: PhaseMetaConfig,
    pub(crate) init: Option<PhaseConfig>,
    pub(crate) reset: Option<PhaseConfig>,
    pub(crate) test: Option<PhaseConfig>,
    pub(crate) files: FileSourceConfig,
    #[serde(default)]
    pub(crate) mutate: HashMap<LanguageId, MutateLanguageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threads: default_threads(),
            bough_dir: default_bough_dir(),
            source_dir: PathBuf::default(),
            ordering: OrderingConfig::default(),
            treat_timeouts_as: Outcome::Caught,
            test_ids: None,
            meta_defaults: PhaseMetaConfig::default(),
            init: None,
            reset: None,
            test: None,
            files: FileSourceConfig::default(),
            mutate: HashMap::new(),
        }
    }
}

// core[impl config.partials]
pub struct ConfigBuilder {
    value: toml::Value,
    source_dir: PathBuf,
}

impl ConfigBuilder {
    pub fn new(source_dir: PathBuf) -> Self {
        Self {
            value: toml::Value::Table(Default::default()),
            source_dir,
        }
    }

    pub fn from_value(self, value: toml::Value) -> Self {
        Self { value, ..self }
    }

    pub fn override_with(mut self, patch: toml::Value) -> Self {
        self.value = Self::deep_merge(self.value, patch);
        self
    }

    fn deep_merge(base: toml::Value, patch: toml::Value) -> toml::Value {
        match (base, patch) {
            (toml::Value::Table(mut base_map), toml::Value::Table(patch_map)) => {
                for (k, v) in patch_map {
                    let merged = match base_map.remove(&k) {
                        Some(existing) => Self::deep_merge(existing, v),
                        None => v,
                    };
                    base_map.insert(k, merged);
                }
                toml::Value::Table(base_map)
            }
            (_, patch) => patch,
        }
    }

    // core[impl config.source-dir]
    pub fn build(self) -> Result<Config, ConfigError> {
        let mut config = Config::deserialize(self.value).map_err(ConfigError::Deserialize)?;
        config.source_dir = self.source_dir;
        Ok(config)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Deserialize(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Deserialize(e) => write!(f, "failed to deserialize config: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

#[cfg(test)]
mod tests {
    use super::*;

    const IDEAL_CONFIG: &str = r#"
threads = 1
bough_dir = "./.bough"
ordering = "alphabetical"
treat_timeouts_as = "Missed"

[init]
pwd = "./examples/vitest"
command = "npm install"

[reset]
pwd = "./examples/vitest"
command = "npm clean"

[test]
pwd = "./examples/vitest"
timeout.absolute = 30
timeout.relative = 3
env = { NODE_ENV = "production" }
command = "npx run test"

[mutate.js]
files.include = ["**/*.js", "**/*.jsx"]
files.exclude = ["**/*__mocks__*"]
mutants.skip = [
  { query = "((fn 'describe') @ignore)" },
  { kind = { "BinaryOp" = "Add" } }
]

[mutate.ts]
files.include = ["**/*.rs"]
mutants.skip = [
  { query = "((condition 'describe') @ignore)" },
  { kind = { "BinaryOp" = "Add" } }
]
"#;

    const MINIMAL_CONFIG: &str = r#"
[test]
command = "npx vitest run"

[mutate.ts]
files.include = ["src/**/*.ts"]
"#;

    fn toml_to_value(s: &str) -> toml::Value {
        toml::from_str(s).unwrap()
    }

    #[test]
    fn deserialize_ideal_config() {
        let config: Config = toml::from_str(IDEAL_CONFIG).expect("failed to parse ideal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert_eq!(config.ordering, OrderingConfig::Alphabetical);
        assert_eq!(config.treat_timeouts_as, Outcome::Missed);

        let init = config.init.as_ref().unwrap();
        assert_eq!(init.meta.pwd, Some(PathBuf::from("./examples/vitest")));
        assert_eq!(init.command, "npm install");

        let test = config.test.as_ref().unwrap();
        assert_eq!(test.meta.timeout.absolute, Some(30));
        assert_eq!(test.meta.timeout.relative, Some(3));
        assert_eq!(test.meta.env["NODE_ENV"], "production");
        assert_eq!(test.command, "npx run test");

        let js_mutate = &config.mutate[&LanguageId::Javascript];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);
    }

    #[test]
    fn deserialize_minimal_config() {
        let config: Config =
            toml::from_str(MINIMAL_CONFIG).expect("failed to parse minimal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert_eq!(config.ordering, OrderingConfig::Random);
        assert_eq!(config.treat_timeouts_as, Outcome::Caught);
        assert!(config.init.is_none());
        assert!(config.reset.is_none());

        let test = config.test.as_ref().unwrap();
        assert_eq!(test.command, "npx vitest run");
        assert_eq!(test.meta.pwd, None);
        assert!(test.meta.env.is_empty());
        assert_eq!(test.meta.timeout, TimeoutConfig::default());

        let ts_mutate = &config.mutate[&LanguageId::Typescript];
        assert_eq!(ts_mutate.files.include, vec!["src/**/*.ts"]);
        assert!(ts_mutate.files.exclude.is_empty());
        assert!(ts_mutate.mutants.skip.is_empty());
    }

    fn build_with_override(base: &str, patch: &str) -> Config {
        ConfigBuilder::new(PathBuf::new())
            .from_value(toml_to_value(base))
            .override_with(toml_to_value(patch))
            .build()
            .unwrap()
    }

    // core[verify config.partials]
    #[test]
    fn override_scalar() {
        let config = build_with_override("", "threads = 4");
        assert_eq!(config.threads, 4);
    }

    // core[verify config.partials]
    #[test]
    fn override_bough_dir() {
        let config = build_with_override(IDEAL_CONFIG, r#"bough_dir = "/override""#);
        assert_eq!(config.bough_dir, PathBuf::from("/override"));
    }

    // core[verify config.partials]
    #[test]
    fn override_vec_replaces() {
        let config = build_with_override(
            MINIMAL_CONFIG,
            r#"
            [test]
            command = "npm test"
            "#,
        );
        assert_eq!(config.test.as_ref().unwrap().command, "npm test");
    }

    // core[verify config.partials]
    #[test]
    fn override_map_adds() {
        let config = build_with_override(
            MINIMAL_CONFIG,
            r#"
            [test.env]
            FOO = "bar"
            "#,
        );
        assert_eq!(config.test.as_ref().unwrap().meta.env["FOO"], "bar");
    }

    // core[verify config.partials]
    #[test]
    fn override_deep_merge_phase() {
        let config = build_with_override(IDEAL_CONFIG, r#"treat_timeouts_as = "Caught""#);
        assert_eq!(config.treat_timeouts_as, Outcome::Caught);
        assert!(config.init.is_some());
    }

    #[test]
    fn defaults_are_sane() {
        let config: Config = toml::from_str("").expect("empty config should parse");
        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert!(config.init.is_none() && config.reset.is_none() && config.test.is_none());
    }

    // core[verify config.source-dir]
    #[test]
    fn project_bough_config_parses() {
        let toml_str = include_str!("../../../bough.config.toml");
        ConfigBuilder::new(PathBuf::new())
            .from_value(toml_to_value(toml_str))
            .build()
            .expect("bough.config.toml should parse without errors or invariant violations");
    }
}

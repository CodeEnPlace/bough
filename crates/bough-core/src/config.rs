use facet::Facet;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, trace, warn};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Facet)]
#[facet(rename_all = "PascalCase")]
#[repr(u8)]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Facet,
    clap::ValueEnum,
    bough_typed_hash::HashInto,
)]
#[facet(rename_all = "lowercase")]
#[repr(u8)]
pub enum LanguageId {
    #[facet(rename = "js")]
    #[value(alias = "js")]
    Javascript,
    #[facet(rename = "ts")]
    #[value(alias = "ts")]
    Typescript,
}

#[derive(Debug, Default, Clone, PartialEq, Facet)]
#[facet(default)]
pub struct PhaseConfig {
    pub(crate) pwd: Option<PathBuf>,
    pub(crate) timeout: TimeoutConfig,
    pub(crate) env: HashMap<String, String>,
    pub(crate) command: String,
}

impl PhaseConfig {
    pub fn command(&self) -> &str {
        &self.command
    }
}

#[derive(Debug, Default, Clone, PartialEq, Facet)]
#[facet(default)]
pub struct TimeoutConfig {
    pub(crate) absolute: Option<u64>,
    pub(crate) relative: Option<u64>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Facet, clap::ValueEnum)]
#[facet(rename_all = "lowercase")]
#[repr(u8)]
pub enum OrderingConfig {
    #[default]
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct TestIdsConfig {
    pub(crate) get_all: PhaseConfig,
    pub(crate) get_failed: PhaseConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Facet)]
#[facet(default)]
pub struct MutateLanguageConfig {
    pub(crate) files: FileSourceConfig,
    pub(crate) mutants: MutantFilterConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Facet)]
#[facet(default)]
pub struct FileSourceConfig {
    pub(crate) include: Vec<String>,
    pub(crate) exclude: Vec<String>,
    pub(crate) ignore_files: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, PartialEq, Facet)]
#[facet(default)]
pub struct MutantFilterConfig {
    pub(crate) skip: Vec<MutantSkipConfig>,
}

#[derive(Debug, Clone, PartialEq, Facet)]
#[facet(untagged)]
#[repr(C)]
pub enum MutantSkipConfig {
    Query { query: String },
    Kind { kind: HashMap<String, String> },
}

#[derive(Debug, Clone, PartialEq, Facet)]
pub struct Config {
    #[facet(default = 1u32)]
    pub(crate) threads: u32,
    #[facet(default = PathBuf::from("./.bough"))]
    pub(crate) bough_dir: PathBuf,
    #[facet(skip, default = PathBuf::new())]
    pub(crate) source_dir: PathBuf,
    #[facet(default)]
    pub(crate) ordering: OrderingConfig,
    #[facet(default = Outcome::Caught)]
    pub(crate) treat_timeouts_as: Outcome,
    pub(crate) pwd: Option<PathBuf>,
    #[facet(default)]
    pub(crate) timeout: TimeoutConfig,
    pub(crate) env: HashMap<String, String>,
    pub(crate) init: Option<PhaseConfig>,
    pub(crate) reset: Option<PhaseConfig>,
    pub(crate) test: Option<PhaseConfig>,
    pub(crate) test_ids: Option<TestIdsConfig>,
    #[facet(default)]
    pub(crate) files: FileSourceConfig,
    pub(crate) mutate: HashMap<LanguageId, MutateLanguageConfig>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            threads: 1,
            bough_dir: PathBuf::from("./.bough"),
            source_dir: PathBuf::default(),
            ordering: OrderingConfig::default(),
            treat_timeouts_as: Outcome::Caught,
            test_ids: None,
            pwd: None,
            timeout: TimeoutConfig::default(),
            env: HashMap::new(),
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
    value: facet_value::Value,
    source_dir: PathBuf,
}

impl ConfigBuilder {
    pub fn new(source_dir: PathBuf) -> Self {
        Self {
            value: facet_value::Value::from_iter(std::iter::empty::<(&str, facet_value::Value)>()),
            source_dir,
        }
    }

    pub fn from_toml(self, toml_str: &str) -> Result<Self, ConfigError> {
        debug!("parsing base config from TOML");
        let value: facet_value::Value =
            facet_toml::from_str(toml_str).map_err(|e| ConfigError::Deserialize(e.to_string()))?;
        Ok(Self { value, ..self })
    }

    pub fn override_with_toml(self, toml_str: &str) -> Result<Self, ConfigError> {
        debug!("applying TOML override");
        let patch: facet_value::Value =
            facet_toml::from_str(toml_str).map_err(|e| ConfigError::Deserialize(e.to_string()))?;
        Ok(Self {
            value: Self::deep_merge(self.value, patch),
            ..self
        })
    }

    fn deep_merge(base: facet_value::Value, patch: facet_value::Value) -> facet_value::Value {
        trace!("deep merging config values");
        match (base.as_object(), patch.as_object()) {
            (Some(base_obj), Some(patch_obj)) => {
                let mut merged = base_obj.clone();
                for (k, v) in patch_obj.iter() {
                    let merged_val = if let Some(existing) = merged.remove(k) {
                        Self::deep_merge(existing, v.clone())
                    } else {
                        v.clone()
                    };
                    merged.insert(k.clone(), merged_val);
                }
                facet_value::Value::from(merged)
            }
            _ => patch,
        }
    }

    // core[impl config.source-dir]
    pub fn build(self) -> Result<Config, ConfigError> {
        let json_str = facet_json::to_string(&self.value)
            .map_err(|e| ConfigError::Serialize(e.to_string()))?;
        let mut config: Config =
            facet_json::from_str(&json_str).map_err(|e| ConfigError::Deserialize(e.to_string()))?;
        config.source_dir = self.source_dir;
        debug!(
            threads = config.threads,
            source_dir = %config.source_dir.display(),
            "config built"
        );
        Ok(config)
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Deserialize(String),
    Serialize(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Deserialize(e) => write!(f, "failed to deserialize config: {e}"),
            ConfigError::Serialize(e) => write!(f, "failed to serialize config: {e}"),
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

    fn builder_from(s: &str) -> ConfigBuilder {
        ConfigBuilder::new(PathBuf::new()).from_toml(s).unwrap()
    }

    #[test]
    fn deserialize_ideal_config() {
        let config: Config =
            facet_toml::from_str(IDEAL_CONFIG).expect("failed to parse ideal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert_eq!(config.ordering, OrderingConfig::Alphabetical);
        assert_eq!(config.treat_timeouts_as, Outcome::Missed);

        let init = config.init.as_ref().unwrap();
        assert_eq!(init.pwd, Some(PathBuf::from("./examples/vitest")));
        assert_eq!(init.command, "npm install");

        let test = config.test.as_ref().unwrap();
        assert_eq!(test.timeout.absolute, Some(30));
        assert_eq!(test.timeout.relative, Some(3));
        assert_eq!(test.env["NODE_ENV"], "production");
        assert_eq!(test.command, "npx run test");

        let js_mutate = &config.mutate[&LanguageId::Javascript];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);
    }

    #[test]
    fn deserialize_minimal_config() {
        let config: Config =
            facet_toml::from_str(MINIMAL_CONFIG).expect("failed to parse minimal config");

        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert_eq!(config.ordering, OrderingConfig::Random);
        assert_eq!(config.treat_timeouts_as, Outcome::Caught);
        assert!(config.init.is_none());
        assert!(config.reset.is_none());

        let test = config.test.as_ref().unwrap();
        assert_eq!(test.command, "npx vitest run");
        assert_eq!(test.pwd, None);
        assert!(test.env.is_empty());
        assert_eq!(test.timeout, TimeoutConfig::default());

        let ts_mutate = &config.mutate[&LanguageId::Typescript];
        assert_eq!(ts_mutate.files.include, vec!["src/**/*.ts"]);
        assert!(ts_mutate.files.exclude.is_empty());
        assert!(ts_mutate.mutants.skip.is_empty());
    }

    fn build_with_override(base: &str, patch: &str) -> Config {
        builder_from(base)
            .override_with_toml(patch)
            .unwrap()
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
        assert_eq!(config.test.as_ref().unwrap().env["FOO"], "bar");
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
        let config: Config = facet_toml::from_str("").expect("empty config should parse");
        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert!(config.init.is_none() && config.reset.is_none() && config.test.is_none());
    }

    // core[verify config.source-dir]
    #[test]
    fn project_bough_config_parses() {
        let toml_str = include_str!("../../../bough.config.toml");
        builder_from(toml_str)
            .build()
            .expect("bough.config.toml should parse without errors or invariant violations");
    }
}

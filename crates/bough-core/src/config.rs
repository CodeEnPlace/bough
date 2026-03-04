use facet::Facet;
use std::collections::HashMap;
use std::path::PathBuf;

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
    bough_typed_hash::HashInto,
)]
#[facet(rename_all = "lowercase")]
#[repr(u8)]
pub enum LanguageId {
    #[facet(rename = "js")]
    Javascript,
    #[facet(rename = "ts")]
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Facet)]
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
#[facet(default)]
pub struct Config {
    #[facet(default = 1u32)]
    pub(crate) threads: u32,
    #[facet(default = PathBuf::from("./.bough"))]
    pub(crate) bough_dir: PathBuf,
    #[facet(default = PathBuf::new())]
    pub(crate) source_dir: PathBuf,
    #[facet(default)]
    pub(crate) ordering: OrderingConfig,
    #[facet(default = Outcome::Caught)]
    pub(crate) treat_timeouts_as: Outcome,
    pub(crate) pwd: Option<PathBuf>,
    #[facet(default)]
    pub(crate) timeout: TimeoutConfig,
    #[facet(default)]
    pub(crate) env: HashMap<String, String>,
    pub(crate) init: Option<PhaseConfig>,
    pub(crate) reset: Option<PhaseConfig>,
    pub(crate) test: Option<PhaseConfig>,
    pub(crate) test_ids: Option<TestIdsConfig>,
    #[facet(default)]
    pub(crate) files: FileSourceConfig,
    #[facet(default)]
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

pub struct TomlFormat;

impl figue::ConfigFormat for TomlFormat {
    fn extensions(&self) -> &[&str] {
        &["toml"]
    }

    fn parse(&self, contents: &str) -> Result<figue::ConfigValue, figue::ConfigFormatError> {
        let value: facet_value::Value = facet_toml::from_str(contents)
            .map_err(|e| figue::ConfigFormatError::new(e.to_string()))?;
        let json_str = facet_json::to_string(&value)
            .map_err(|e| figue::ConfigFormatError::new(e.to_string()))?;
        figue::JsonFormat
            .parse(&json_str)
            .map_err(|e| figue::ConfigFormatError::new(e.to_string()))
    }
}

#[derive(Debug, Facet)]
pub struct Args {
    #[facet(figue::config)]
    pub config: Config,
}

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

    fn parse_toml(s: &str) -> Config {
        facet_toml::from_str(s).expect("failed to parse toml")
    }

    fn parse_with_cli(toml: &str, cli_args: &[&str]) -> Args {
        let config = figue::builder::<Args>()
            .unwrap()
            .file(|f| f.format(TomlFormat).content(toml, "config.toml"))
            .cli(|cli| cli.args(cli_args.iter().copied()))
            .build();
        figue::Driver::new(config)
            .run()
            .into_result()
            .unwrap()
            .value
    }

    #[test]
    fn deserialize_ideal_config() {
        let config = parse_toml(IDEAL_CONFIG);

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
        let config = parse_toml(MINIMAL_CONFIG);

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

    #[test]
    fn defaults_are_sane() {
        let config = parse_toml("");
        assert_eq!(config.threads, 1);
        assert_eq!(config.bough_dir, PathBuf::from("./.bough"));
        assert!(config.init.is_none() && config.reset.is_none() && config.test.is_none());
    }

    #[test]
    fn project_bough_config_parses() {
        let toml_str = include_str!("../../../bough.config.toml");
        parse_toml(toml_str);
    }

    #[test]
    fn figue_defaults_only() {
        let config = figue::builder::<Args>()
            .unwrap()
            .cli(|cli| cli.args(std::iter::empty::<&str>()))
            .build();
        let result = figue::Driver::new(config).run().into_result();
        assert!(result.is_ok(), "figue defaults failed: {:?}", result.err());
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_shallow_scalar() {
        let args = parse_with_cli("", &["--config.threads", "4"]);
        assert_eq!(args.config.threads, 4);
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_shallow_path() {
        let args = parse_with_cli(IDEAL_CONFIG, &["--config.bough-dir", "/override"]);
        assert_eq!(args.config.bough_dir, PathBuf::from("/override"));
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_shallow_enum() {
        let args = parse_with_cli(IDEAL_CONFIG, &["--config.ordering", "alphabetical"]);
        assert_eq!(args.config.ordering, OrderingConfig::Alphabetical);
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_deep_nested_command() {
        let args = parse_with_cli(MINIMAL_CONFIG, &["--config.test.command", "npm test"]);
        assert_eq!(args.config.test.as_ref().unwrap().command, "npm test");
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_deep_timeout() {
        let args = parse_with_cli(
            IDEAL_CONFIG,
            &["--config.test.timeout.absolute", "60"],
        );
        assert_eq!(args.config.test.as_ref().unwrap().timeout.absolute, Some(60));
        assert_eq!(args.config.test.as_ref().unwrap().timeout.relative, Some(3));
    }

    // core[verify config.partials]
    #[test]
    fn cli_override_preserves_unrelated_fields() {
        let args = parse_with_cli(IDEAL_CONFIG, &["--config.threads", "8"]);
        assert_eq!(args.config.threads, 8);
        assert!(args.config.init.is_some());
        assert_eq!(args.config.ordering, OrderingConfig::Alphabetical);
    }
}

use std::collections::HashMap;
use std::env;

use facet::Facet;
use figue::{self as args, ConfigFormat, ConfigFormatError, Driver, builder};
use miette::Diagnostic;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Facet, Debug)]
pub struct Cli {
    #[facet(args::named, args::short = 'v', args::counted, default)]
    pub verbose: u8,

    #[facet(args::subcommand)]
    pub command: Command,

    #[facet(args::config, args::env_prefix = "BOUGH")]
    pub config: Config,
}

#[derive(Facet, Debug)]
#[repr(u8)]
pub enum Command {
    Show {
        #[facet(args::subcommand)]
        what: ShowCommand,
    },
    Run,
}

#[derive(Facet, Debug)]
#[repr(u8)]
pub enum ShowCommand {
    Cli,
    Config,
    File,
}

#[derive(Facet, Debug, Clone)]
pub struct Config {
    #[facet(default = 1)]
    pub workers: u64,

    #[facet(default = 1)]
    pub threads: u64,

    pub include: Vec<String>,

    pub exclude: Vec<String>,

    pub lang: HashMap<bough_core::LanguageId, LanguageConfig>,
}

#[derive(Facet, Debug, Clone)]
pub struct LanguageConfig {
    pub include: Vec<String>,

    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum Error {
    #[error("config.include must not be empty")]
    #[diagnostic(
        code(bough::config::empty_include),
        help("add at least one include glob pattern")
    )]
    EmptyInclude,

    #[error("at least one language must be configured")]
    #[diagnostic(
        code(bough::config::no_languages),
        help("add a [js] or [ts] section to your config")
    )]
    NoLanguages,

    #[error("{0}")]
    #[diagnostic(code(bough::config::parse))]
    Parse(String),
}

impl Cli {
    pub fn validate(&self) -> Vec<Error> {
        let mut errors = Vec::new();
        if self.config.include.is_empty() {
            errors.push(Error::EmptyInclude);
        }
        if self.config.lang.is_empty() {
            errors.push(Error::NoLanguages);
        }
        errors
    }
}

struct TomlFormat;

impl ConfigFormat for TomlFormat {
    fn extensions(&self) -> &[&str] {
        &["toml"]
    }

    fn parse(&self, contents: &str) -> Result<figue::ConfigValue, ConfigFormatError> {
        facet_toml::from_str(contents).map_err(|e| ConfigFormatError::new(e.to_string()))
    }
}

struct YamlFormat;

impl ConfigFormat for YamlFormat {
    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }

    fn parse(&self, contents: &str) -> Result<figue::ConfigValue, ConfigFormatError> {
        facet_yaml::from_str(contents).map_err(|e| ConfigFormatError::new(e.to_string()))
    }
}

const CONFIG_NAMES: &[&str] = &[
    "bough.config.toml",
    "bough.config.yaml",
    "bough.config.yml",
    "bough.config.json",
    ".bough.toml",
    ".bough.yaml",
    ".bough.yml",
    ".bough.json",
    ".config/bough.toml",
    ".config/bough.yaml",
    ".config/bough.yml",
    ".config/bough.json",
];

fn find_config_paths() -> Vec<String> {
    let mut paths = Vec::new();
    let mut dir = env::current_dir().ok();
    while let Some(d) = dir {
        for name in CONFIG_NAMES {
            paths.push(d.join(name).to_string_lossy().into_owned());
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    debug!(count = paths.len(), "searched config paths");
    paths
}

pub fn resolve_config_path() -> Option<String> {
    let result = find_config_paths()
        .into_iter()
        .find(|p| std::path::Path::new(p).is_file());
    match &result {
        Some(path) => info!(path, "resolved config file"),
        None => warn!("no config file found"),
    }
    result
}

pub fn parse() -> Cli {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .env(|env| env)
        .file(|f| {
            f.default_paths(find_config_paths())
                .format(TomlFormat)
                .format(YamlFormat)
        })
        .build();

    let outcome = Driver::new(config).run();
    let output = match outcome.into_result() {
        Ok(output) => output,
        Err(figue::DriverError::Help { text }) => {
            println!("{text}");
            std::process::exit(0);
        }
        Err(figue::DriverError::Failed { report }) => {
            eprintln!("{}", report.render_pretty());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
    };
    output.print_warnings();
    let cli = output.get_silent();

    let errors = cli.validate();
    if !errors.is_empty() {
        warn!(count = errors.len(), "config validation failed");
        for error in &errors {
            eprintln!("{:?}", miette::Report::new_boxed(Box::new(error.clone())));
        }
        std::process::exit(1);
    }

    debug!(
        workers = cli.config.workers,
        verbose = cli.verbose,
        "config parsed"
    );
    cli
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL_TOML: &str = r#"
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []
"#;

    const FULL_TOML: &str = r#"
workers = 16
include = ["src/**", "lib/**"]
exclude = ["target/**"]

[lang.js]
include = ["**/*.js"]
exclude = ["node_modules/**"]

[lang.ts]
include = ["**/*.ts"]
exclude = []
"#;

    pub fn try_parse_from(
        args: &[&str],
        config_content: Option<(&str, &str)>,
    ) -> Result<Cli, Vec<Error>> {
        let b = builder::<Cli>()
            .expect("schema should be valid")
            .cli(|cli| cli.args(args.iter().map(|s| s.to_string())));

        let b = match config_content {
            Some((content, filename)) => b.file(|f| {
                f.content(content, filename)
                    .format(TomlFormat)
                    .format(YamlFormat)
            }),
            None => b,
        };

        let config = b.build();
        let cli: Cli = Driver::new(config)
            .run()
            .into_result()
            .map_err(|e| vec![Error::Parse(format!("{e:?}"))])?
            .get_silent();

        let errors = cli.validate();
        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(cli)
    }

    fn parse_ok(args: &[&str], toml: &str) -> Cli {
        try_parse_from(args, Some((toml, "config.toml"))).expect("should parse")
    }

    fn parse_err(args: &[&str], toml: &str) -> Vec<Error> {
        try_parse_from(args, Some((toml, "config.toml"))).expect_err("should fail")
    }

    #[test]
    fn defaults_with_minimal_config() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(cli.config.workers, 1);
        assert_eq!(cli.config.include, vec!["src/**"]);
        assert!(cli.config.exclude.is_empty());
        assert_eq!(cli.verbose, 0);
    }

    #[test]
    fn full_config() {
        let cli = parse_ok(&["run"], FULL_TOML);
        assert_eq!(cli.config.workers, 16);
        assert_eq!(cli.config.include, vec!["src/**", "lib/**"]);
        assert_eq!(cli.config.exclude, vec!["target/**"]);
    }

    #[test]
    fn cli_overrides_file() {
        let cli = parse_ok(&["run", "--config.workers", "32"], FULL_TOML);
        assert_eq!(cli.config.workers, 32);
        assert_eq!(cli.config.include, vec!["src/**", "lib/**"]);
    }

    #[test]
    fn verbose_counted() {
        let cli = parse_ok(&["-vvv", "run"], MINIMAL_TOML);
        assert_eq!(cli.verbose, 3);
    }

    #[test]
    fn show_config_subcommand() {
        let cli = parse_ok(&["show", "config"], MINIMAL_TOML);
        assert!(matches!(
            cli.command,
            Command::Show {
                what: ShowCommand::Config
            }
        ));
    }

    #[test]
    fn show_file_subcommand() {
        let cli = parse_ok(&["show", "file"], MINIMAL_TOML);
        assert!(matches!(
            cli.command,
            Command::Show {
                what: ShowCommand::File
            }
        ));
    }

    #[test]
    fn show_cli_subcommand() {
        let cli = parse_ok(&["show", "cli"], MINIMAL_TOML);
        assert!(matches!(
            cli.command,
            Command::Show {
                what: ShowCommand::Cli
            }
        ));
    }

    #[test]
    fn run_subcommand() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert!(matches!(cli.command, Command::Run));
    }

    #[test]
    fn empty_include_fails_validation() {
        let toml = r#"
include = []
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []
"#;
        let errors = parse_err(&["run"], toml);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], Error::EmptyInclude));
    }

    #[test]
    fn missing_include_fails_parse() {
        let toml = r#"
exclude = []
"#;
        let errors = parse_err(&["run"], toml);
        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], Error::Parse(_)));
    }

    #[test]
    fn language_config_from_toml() {
        let cli = parse_ok(&["run"], FULL_TOML);
        assert_eq!(cli.config.lang.len(), 2);

        let js = &cli.config.lang[&bough_core::LanguageId::Javascript];
        assert_eq!(js.include, vec!["**/*.js"]);
        assert_eq!(js.exclude, vec!["node_modules/**"]);

        let ts = &cli.config.lang[&bough_core::LanguageId::Typescript];
        assert_eq!(ts.include, vec!["**/*.ts"]);
        assert!(ts.exclude.is_empty());
    }

    #[test]
    fn missing_lang_fails_parse() {
        let toml = r#"
include = ["src/**"]
exclude = []
"#;
        let errors = parse_err(&["run"], toml);
        assert!(errors.iter().any(|e| matches!(e, Error::Parse(_))));
    }

    #[test]
    fn empty_lang_fails_validation() {
        let toml = r#"
include = ["src/**"]
exclude = []

[lang]
"#;
        let errors = parse_err(&["run"], toml);
        assert!(errors.iter().any(|e| matches!(e, Error::NoLanguages)));
    }

    #[test]
    fn json_config() {
        let json = r#"{"include": ["src/**"], "exclude": [], "lang": {"js": {"include": ["**/*.js"], "exclude": []}}}"#;
        let cli = try_parse_from(&["run"], Some((json, "config.json"))).expect("should parse");
        assert_eq!(cli.config.include, vec!["src/**"]);
    }

    #[test]
    fn lang_include_globs_no_base() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<&str> = bough_core::Config::get_lang_include_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert_eq!(globs, vec!["**/*.js"]);
    }

    #[test]
    fn lang_exclude_globs_prepend_base() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<&str> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert_eq!(globs, vec!["target/**", "node_modules/**"]);
    }

    #[test]
    fn lang_include_globs_lang_only() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<&str> = bough_core::Config::get_lang_include_globs(
            &cli.config,
            bough_core::LanguageId::Typescript,
        )
        .collect();
        assert_eq!(globs, vec!["**/*.ts"]);
    }

    #[test]
    fn lang_exclude_globs_base_only_when_lang_empty() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<&str> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Typescript,
        )
        .collect();
        assert_eq!(globs, vec!["target/**"]);
    }

    #[test]
    fn lang_globs_with_no_base_excludes() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        let globs: Vec<&str> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert!(globs.is_empty());
    }

    #[test]
    fn yaml_config() {
        let yaml = "include:\n  - \"src/**\"\nexclude: []\nlang:\n  js:\n    include:\n      - \"**/*.js\"\n    exclude: []\n";
        let cli = try_parse_from(&["run"], Some((yaml, "config.yaml"))).expect("should parse");
        assert_eq!(cli.config.include, vec!["src/**"]);
    }
}

impl bough_core::Config for Config {
    fn get_workers_count(&self) -> u64 {
        self.workers
    }

    fn get_bough_state_dir(&self) -> std::path::PathBuf {
        self.get_base_root_path().join(".bough")
    }

    fn get_base_root_path(&self) -> std::path::PathBuf {
        resolve_config_path()
            .map(|p| {
                std::path::Path::new(&p)
                    .parent()
                    .unwrap_or(std::path::Path::new("."))
                    .to_path_buf()
            })
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| ".".into()))
    }

    fn get_base_include_globs(&self) -> impl Iterator<Item = &str> {
        self.include.iter().map(|s| s.as_str())
    }

    fn get_base_exclude_globs(&self) -> impl Iterator<Item = &str> {
        self.exclude.iter().map(|s| s.as_str())
    }

    fn get_langs(&self) -> impl Iterator<Item = bough_core::LanguageId> {
        self.lang.keys().copied().collect::<Vec<_>>().into_iter()
    }

    fn get_lang_include_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = &str> {
        self.lang
            .get(&language_id)
            .map(|c| c.include.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .unwrap_or_default()
            .into_iter()
    }

    fn get_lang_exclude_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = &str> {
        self.exclude
            .iter()
            .map(|s| s.as_str())
            .chain(
                self.lang
                    .get(&language_id)
                    .map(|c| c.exclude.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>()
            .into_iter()
    }
}

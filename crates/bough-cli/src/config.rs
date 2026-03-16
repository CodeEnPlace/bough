use bough_core::Factor;
use facet::Facet;
use figue::{self as args, ConfigFormat, ConfigFormatError, Driver, builder};
use miette::Diagnostic;
use std::env;
use std::io::IsTerminal;
use std::{collections::HashMap, path::PathBuf};
use thiserror::Error;
use tracing::{debug, warn};

/// Bough — a polyglot mutation testing tool.
#[derive(Facet, Debug, Clone)]
pub struct Cli {
    /// Increase log verbosity. Repeat for more detail: `-v` info, `-vv` debug, `-vvv` trace.
    #[facet(args::named, args::short = 'v', args::counted, default)]
    pub verbose: u8,

    /// Output format. One of: terse, verbose, markdown, json. Default: terse.
    #[facet(args::named, args::short = 'f', default)]
    pub format: Format,

    /// Disable colored output. Also honoured via the `NO_COLOR` environment variable.
    #[facet(args::named, default)]
    no_color: bool,

    /// Subcommand to run. See `bough <command> --help` for details.
    #[facet(args::subcommand)]
    pub command: Command,

    /// Configuration values. Sources in override order: config file →
    /// `BOUGH_*` env vars → `--config.*` CLI flags.
    #[facet(args::config, args::env_prefix = "BOUGH")]
    pub config: Config,
}

pub fn resolve_color(no_color_flag: bool, is_tty: bool) -> bool {
    if no_color_flag || env::var("NO_COLOR").is_ok() {
        return false;
    }
    is_tty
}

impl Cli {
    pub fn color(&self) -> bool {
        resolve_color(self.no_color, std::io::stdout().is_terminal())
    }
}

/// Output format for command results.
#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Format {
    /// Compact single-line output. Default.
    #[default]
    #[facet(rename = "terse")]
    Terse,
    /// Human-readable multi-line output with extra detail.
    #[facet(rename = "verbose")]
    Verbose,
    /// Markdown-formatted output, suitable for reports or piping to a renderer.
    #[facet(rename = "markdown")]
    Markdown,
    /// Machine-readable JSON output.
    #[facet(rename = "json")]
    Json,
}

/// Top-level subcommands.
#[derive(Facet, Debug, Clone)]
#[repr(u8)]
pub enum Command {
    Show {
        #[facet(args::subcommand)]
        show: Show,
    },

    Step {
        #[facet(args::subcommand)]
        step: Step,
    },

    Find {
        #[facet(args::positional, default)]
        lang: Option<bough_core::LanguageId>,
        #[facet(args::positional, default)]
        file: Option<PathBuf>,
    },

    Run,
    Noop,
}

#[derive(Facet, Debug, Clone)]
#[repr(u8)]
pub enum Show {
    Config,

    Files {
        #[facet(args::positional, default)]
        lang: Option<bough_core::LanguageId>,
    },

    Mutations {
        #[facet(args::positional, default)]
        lang: Option<bough_core::LanguageId>,
        #[facet(args::positional, default)]
        file: Option<PathBuf>,
    },

    Mutation {
        #[facet(args::positional)]
        hash: String,
    },
}

#[derive(Facet, Debug, Clone)]
#[repr(u8)]
pub enum Step {
    TendState,

    TendWorkspaces,

    InitWorkspace {
        #[facet(args::positional)]
        workspace_id: String,
    },

    ResetWorkspace {
        #[facet(args::positional)]
        workspace_id: String,
    },

    ApplyMutation {
        #[facet(args::positional)]
        workspace_id: String,
        #[facet(args::positional)]
        mutation_hash: String,
    },

    UnapplyMutation {
        #[facet(args::positional)]
        workspace_id: String,
        #[facet(args::positional)]
        mutation_hash: String,
    },

    TestMutation {
        #[facet(args::positional)]
        workspace_id: String,
        #[facet(args::positional)]
        mutation_hash: String,
    },
}

#[derive(Facet, Debug, Clone)]
pub struct Config {
    #[facet(default = 1)]
    pub workers: u64,

    #[facet(default = 1)]
    pub threads: u64,

    // bough[impl config.base-root-path.default]
    pub base_root_dir: String,

    pub include: Vec<String>,

    pub exclude: Vec<String>,

    pub lang: HashMap<bough_core::LanguageId, LanguageConfig>,

    #[facet(flatten, default)]
    pub phase_defaults: PhaseOverrides,

    #[facet(default)]
    pub test: Option<TestPhaseConfig>,

    #[facet(default)]
    pub init: Option<PhaseConfig>,

    #[facet(default)]
    pub reset: Option<PhaseConfig>,

    #[facet(default)]
    pub find: FindMutationsConfig,
}

#[derive(Facet, Debug, Clone)]
pub struct FindMutationsConfig {
    #[facet(default = 1)]
    pub number: usize,

    #[facet(default = 1)]
    pub number_per_file: usize,

    #[facet(default = vec![Factor::EncompasingMissedMutationsCount, Factor::TSNodeDepth])]
    pub factors: Vec<Factor>,
}

#[derive(Facet, Debug, Clone, Default)]
pub struct PhaseOverrides {
    #[facet(default)]
    pub pwd: Option<String>,

    #[facet(default)]
    pub env: Option<HashMap<String, String>>,

    #[facet(default)]
    pub timeout: Option<TimeoutConfig>,
}

#[derive(Facet, Debug, Clone)]
pub struct TimeoutConfig {
    #[facet(default)]
    pub absolute: Option<u64>,

    #[facet(default)]
    pub relative: Option<f64>,
}

#[derive(Facet, Debug, Clone)]
pub struct TestPhaseConfig {
    pub cmd: String,

    #[facet(flatten, default)]
    pub overrides: PhaseOverrides,
}

#[derive(Facet, Debug, Clone)]
pub struct PhaseConfig {
    #[facet(default)]
    pub cmd: Option<String>,

    #[facet(flatten, default)]
    pub overrides: PhaseOverrides,
}

#[derive(Facet, Debug, Clone)]
pub struct LanguageConfig {
    pub include: Vec<String>,

    pub exclude: Vec<String>,

    #[facet(default)]
    pub skip: Option<LanguageSkipConfig>,
}

#[derive(Facet, Debug, Clone, Default)]
pub struct LanguageSkipConfig {
    #[facet(default)]
    pub query: Vec<String>,
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

    #[error("test.cmd is required")]
    #[diagnostic(
        code(bough::config::missing_test_cmd),
        help("add a [test] section with cmd = \"your test command\"")
    )]
    MissingTestCmd,

    #[error("timeout section must specify at least one of 'absolute' or 'relative'{0}")]
    #[diagnostic(
        code(bough::config::empty_timeout),
        help("add absolute = <seconds> and/or relative = <multiplier>")
    )]
    EmptyTimeout(String),

    #[cfg(test)]
    #[error("{0}")]
    #[diagnostic(code(bough::config::parse))]
    Parse(String),
}

impl Cli {
    pub fn validate(&self) -> Vec<Error> {
        let mut errors = Vec::new();
        // bough[impl config.include.at-least-one]
        if self.config.include.is_empty() {
            errors.push(Error::EmptyInclude);
        }
        if self.config.lang.is_empty() {
            errors.push(Error::NoLanguages);
        }
        if self.config.test.is_none() {
            errors.push(Error::MissingTestCmd);
        }
        if let Some(ref t) = self.config.phase_defaults.timeout {
            if t.absolute.is_none() && t.relative.is_none() {
                errors.push(Error::EmptyTimeout("".to_string()));
            }
        }
        for (label, overrides) in self.config.phase_timeout_overrides() {
            if let Some(ref t) = overrides.timeout {
                if t.absolute.is_none() && t.relative.is_none() {
                    errors.push(Error::EmptyTimeout(format!(" (in {label})")));
                }
            }
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

// bough[impl config.exclude.from-vcs-ignore]
// bough[impl config.lang.exclude.from-vcs-ignore]
pub fn collect_vcs_ignore_globs(root: &std::path::Path) -> Vec<String> {
    let mut globs = Vec::new();
    let mut dir = Some(root.to_path_buf());
    while let Some(d) = dir {
        let gitignore = d.join(".gitignore");
        if let Ok(content) = std::fs::read_to_string(&gitignore) {
            debug!(path = %gitignore.display(), "reading vcs ignore file");
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                    continue;
                }
                let pattern = if trimmed.starts_with('/') {
                    trimmed[1..].to_string()
                } else if trimmed.contains('/') {
                    trimmed.to_string()
                } else if trimmed.starts_with("**/") {
                    trimmed.to_string()
                } else {
                    format!("**/{trimmed}")
                };
                globs.push(pattern);
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    globs
}

const VCS_DIRS: &[&str] = &[".git", ".jj", ".hg", ".svn"];

// bough[impl config.exclude.from-vcs-dir]
// bough[impl config.lang.exclude.from-vcs-dir]
pub fn collect_vcs_dir_globs(root: &std::path::Path) -> Vec<String> {
    VCS_DIRS
        .iter()
        .filter(|d| root.join(d).is_dir())
        .map(|d| format!("{d}/**"))
        .collect()
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

fn find_config_candidates_from(start: &std::path::Path) -> Vec<(std::path::PathBuf, String)> {
    let mut candidates = Vec::new();
    let mut dir = Some(start.to_path_buf());
    while let Some(d) = dir {
        for name in CONFIG_NAMES {
            let path = d.join(name).to_string_lossy().into_owned();
            candidates.push((d.clone(), path));
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    debug!(count = candidates.len(), "searched config paths");
    candidates
}

fn find_config_candidates() -> Vec<(std::path::PathBuf, String)> {
    env::current_dir()
        .map(|d| find_config_candidates_from(&d))
        .unwrap_or_default()
}

// bough[impl config.base-root-path.absolutized-relative-to-file]
pub fn resolve_root_path(config_dir: &std::path::Path, root: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(root);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let base = if config_dir.is_absolute() {
        config_dir.to_path_buf()
    } else {
        std::env::current_dir()
            .expect("current directory should be accessible")
            .join(config_dir)
    };
    let joined = base.join(path);
    normalize_path(&joined)
}

fn normalize_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            c => components.push(c),
        }
    }
    components.iter().collect()
}

#[cfg(test)]
fn resolve_config_from(start: &std::path::Path) -> Option<(std::path::PathBuf, String)> {
    find_config_candidates_from(start)
        .into_iter()
        .find(|(_, p)| std::path::Path::new(p).is_file())
}

// bough[impl config.base-root-path.relative-via-figue]
fn config_dir_from_report(report: &figue::DriverReport) -> Option<std::path::PathBuf> {
    let fr = report.file_resolution.as_ref()?;
    let picked = fr
        .paths
        .iter()
        .find(|p| format!("{:?}", p.status).contains("Picked"))?;
    std::path::Path::new(picked.path.as_str())
        .parent()
        .map(|d| d.to_path_buf())
}

pub fn parse() -> Cli {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .env(|env| env)
        .file(|f| {
            f.default_paths(
                find_config_candidates()
                    .into_iter()
                    .map(|(_, p)| p)
                    .collect::<Vec<_>>(),
            )
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
    let (mut cli, report) = output.into_parts();

    // bough[impl config.base-root-path+2]
    // bough[impl config.base-root-path.absolutized-relative-to-file]
    if let Some(config_dir) = config_dir_from_report(&report) {
        cli.config.base_root_dir = resolve_root_path(&config_dir, &cli.config.base_root_dir)
            .to_string_lossy()
            .into_owned();
    }

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
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#;

    const FULL_TOML: &str = r#"
base_root_dir = "."
workers = 16
include = ["src/**", "lib/**"]
exclude = ["target/**"]

[lang.js]
include = ["**/*.js"]
exclude = ["node_modules/**"]

[lang.ts]
include = ["**/*.ts"]
exclude = []

[test]
cmd = "npm test"
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
    fn show_files_subcommand() {
        let cli = parse_ok(&["show", "files"], MINIMAL_TOML);
        assert!(matches!(
            cli.command,
            Command::Show {
                show: Show::Files { .. }
            }
        ));
    }

    #[test]
    fn run_subcommand() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert!(matches!(cli.command, Command::Run));
    }

    #[test]
    fn find_defaults() {
        let cli = parse_ok(&["find"], MINIMAL_TOML);
        match cli.command {
            Command::Find { lang, file } => {
                assert_eq!(lang, None);
                assert_eq!(file, None);
            }
            other => panic!("expected Find, got {other:?}"),
        }
        assert_eq!(cli.config.find.number, 1);
        assert_eq!(cli.config.find.number_per_file, 1);
        assert_eq!(
            cli.config.find.factors,
            vec![Factor::EncompasingMissedMutationsCount, Factor::TSNodeDepth]
        );
    }

    #[test]
    fn find_config_from_file() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"

[find]
number = 5
number_per_file = 3
factors = ["VcsFileChurn", "MutationSeverity"]
"#;
        let cli = parse_ok(&["find"], toml);
        assert_eq!(cli.config.find.number, 5);
        assert_eq!(cli.config.find.number_per_file, 3);
        assert_eq!(
            cli.config.find.factors,
            vec![Factor::VcsFileChurn, Factor::MutationSeverity]
        );
    }

    #[test]
    fn find_with_lang_and_file_args() {
        let cli = parse_ok(&["find", "js", "src/foo.js"], MINIMAL_TOML);
        match cli.command {
            Command::Find { lang, file } => {
                assert_eq!(lang, Some(bough_core::LanguageId::Javascript));
                assert_eq!(file, Some(PathBuf::from("src/foo.js")));
            }
            other => panic!("expected Find, got {other:?}"),
        }
    }

    // bough[verify config.include.at-least-one]
    #[test]
    fn empty_include_fails_validation() {
        let toml = r#"
base_root_dir = "."
include = []
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
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

    // bough[verify config.base-root-path.default]
    #[test]
    fn missing_base_root_dir_fails_parse() {
        let toml = r#"
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []
"#;
        let errors = parse_err(&["run"], toml);
        assert!(errors.iter().any(|e| matches!(e, Error::Parse(_))));
    }

    #[test]
    fn empty_lang_fails_validation() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang]
"#;
        let errors = parse_err(&["run"], toml);
        assert!(errors.iter().any(|e| matches!(e, Error::NoLanguages)));
    }

    #[test]
    fn json_config() {
        let json = r#"{"base_root_dir": ".", "include": ["src/**"], "exclude": [], "lang": {"js": {"include": ["**/*.js"], "exclude": []}}, "test": {"cmd": "echo test"}}"#;
        let cli = try_parse_from(&["run"], Some((json, "config.json"))).expect("should parse");
        assert_eq!(cli.config.include, vec!["src/**"]);
    }

    // bough[verify config.lang.include.derived]
    #[test]
    fn lang_include_globs_does_not_include_base() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_include_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert_eq!(globs, vec!["**/*.js"]);
        assert!(!globs.contains(&"src/**".to_string()));
        assert!(!globs.contains(&"lib/**".to_string()));
    }

    // bough[verify config.lang.exclude.derived]
    #[test]
    fn lang_exclude_globs_prepend_base() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert!(globs.contains(&"target/**".to_string()));
        assert!(globs.contains(&"node_modules/**".to_string()));
        assert!(
            globs.iter().position(|g| g == "target/**")
                < globs.iter().position(|g| g == "node_modules/**")
        );
    }

    #[test]
    fn lang_include_globs_only_lang_specific() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_include_globs(
            &cli.config,
            bough_core::LanguageId::Typescript,
        )
        .collect();
        assert_eq!(globs, vec!["**/*.ts"]);
    }

    #[test]
    fn lang_exclude_globs_base_only_when_lang_empty() {
        let cli = parse_ok(&["run"], FULL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Typescript,
        )
        .collect();
        assert!(globs.contains(&"target/**".to_string()));
        assert_eq!(globs[0], "target/**");
    }

    #[test]
    fn lang_globs_with_no_base_excludes() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert!(globs.iter().all(|g| !MINIMAL_TOML.contains(g.as_str())));
    }

    #[test]
    fn yaml_config() {
        let yaml = "base_root_dir: \".\"\ninclude:\n  - \"src/**\"\nexclude: []\nlang:\n  js:\n    include:\n      - \"**/*.js\"\n    exclude: []\ntest:\n  cmd: \"echo test\"\n";
        let cli = try_parse_from(&["run"], Some((yaml, "config.yaml"))).expect("should parse");
        assert_eq!(cli.config.include, vec!["src/**"]);
    }

    // bough[verify config.exclude.bough-dir]
    #[test]
    fn base_exclude_globs_includes_bough_dir() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        let globs: Vec<String> = bough_core::Config::get_base_exclude_globs(&cli.config).collect();
        assert!(globs.iter().any(|g| g.contains(".bough")));
    }

    // bough[verify config.lang.exclude.bough-dir]
    // bough[verify config.lang.exclude.from-vcs-ignore]
    #[test]
    fn lang_exclude_globs_includes_bough_dir() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        let globs: Vec<String> = bough_core::Config::get_lang_exclude_globs(
            &cli.config,
            bough_core::LanguageId::Javascript,
        )
        .collect();
        assert!(globs.iter().any(|g| g.contains(".bough")));
    }

    // bough[verify config.exclude.from-vcs-dir]
    // bough[verify config.lang.exclude.from-vcs-dir]
    #[test]
    fn collect_vcs_dir_globs_finds_git_dir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        let globs = collect_vcs_dir_globs(dir.path());
        assert!(globs.contains(&".git/**".to_string()));
    }

    // bough[verify config.exclude.from-vcs-dir]
    #[test]
    fn collect_vcs_dir_globs_finds_multiple() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::create_dir_all(dir.path().join(".jj")).unwrap();
        let globs = collect_vcs_dir_globs(dir.path());
        assert!(globs.contains(&".git/**".to_string()));
        assert!(globs.contains(&".jj/**".to_string()));
    }

    // bough[verify config.exclude.from-vcs-dir]
    #[test]
    fn collect_vcs_dir_globs_empty_when_none() {
        let dir = tempfile::tempdir().unwrap();
        let globs = collect_vcs_dir_globs(dir.path());
        assert!(globs.is_empty());
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_reads_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "node_modules\n*.log\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.contains(&"**/node_modules".to_string()));
        assert!(globs.contains(&"**/*.log".to_string()));
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_skips_comments_and_empty_lines() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".gitignore"),
            "# a comment\n\nnode_modules\n  \n",
        )
        .unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert_eq!(globs.len(), 1);
        assert!(globs.contains(&"**/node_modules".to_string()));
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_skips_negation_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "dist\n!dist/keep\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert_eq!(globs, vec!["**/dist"]);
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_handles_slash_prefixed_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "/build\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert_eq!(globs, vec!["build".to_string()]);
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_preserves_glob_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "**/*.o\nsrc/**/*.tmp\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.contains(&"**/*.o".to_string()));
        assert!(globs.contains(&"src/**/*.tmp".to_string()));
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_returns_empty_when_no_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.is_empty());
    }

    // bough[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_reads_parent_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("project");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(dir.path().join(".gitignore"), "*.log\n").unwrap();
        std::fs::write(child.join(".gitignore"), "dist\n").unwrap();
        let globs = collect_vcs_ignore_globs(&child);
        assert!(globs.contains(&"**/dist".to_string()));
        assert!(globs.contains(&"**/*.log".to_string()));
    }

    // bough[verify config.base-root-path.relative-via-figue]
    #[test]
    fn config_dir_extracted_from_figue_report() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bough.config.toml");
        std::fs::write(&config_path, &config_toml_with_root(".")).unwrap();

        let config = builder::<Cli>()
            .expect("schema should be valid")
            .cli(|cli| cli.args(["run"].iter().map(|s| s.to_string())))
            .file(|f| {
                f.default_paths(vec![config_path.to_string_lossy().into_owned()])
                    .format(TomlFormat)
                    .format(YamlFormat)
            })
            .build();

        let output = Driver::new(config)
            .run()
            .into_result()
            .expect("should parse");
        let (_, report) = output.into_parts();
        let config_dir = super::config_dir_from_report(&report);
        assert!(
            config_dir.is_some(),
            "should extract config dir from figue report"
        );
        assert_eq!(config_dir.unwrap(), dir.path());
    }

    fn parse_from_disk(dir: &std::path::Path, config_filename: &str, toml: &str) -> Cli {
        let config_path = dir.join(config_filename);
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&config_path, toml).unwrap();

        let (_, resolved_path) = resolve_config_from(dir).expect("should find config file");

        let config = builder::<Cli>()
            .expect("schema should be valid")
            .cli(|cli| cli.args(["run"].iter().map(|s| s.to_string())))
            .file(|f| {
                f.default_paths(vec![resolved_path.clone()])
                    .format(TomlFormat)
                    .format(YamlFormat)
            })
            .build();

        let mut cli: Cli = Driver::new(config)
            .run()
            .into_result()
            .expect("should parse")
            .get_silent();

        let config_dir = std::path::Path::new(&resolved_path)
            .parent()
            .expect("config file should have a parent directory");
        cli.config.base_root_dir = resolve_root_path(config_dir, &cli.config.base_root_dir)
            .to_string_lossy()
            .into_owned();

        cli
    }

    fn config_toml_with_root(base_root_dir: &str) -> String {
        format!(
            r#"
base_root_dir = "{base_root_dir}"
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "echo test"
"#
        )
    }

    // bough[verify config.base-root-path+2]
    #[test]
    fn base_root_path_from_top_level_config_dot() {
        let dir = tempfile::tempdir().unwrap();
        let cli = parse_from_disk(dir.path(), "bough.config.toml", &config_toml_with_root("."));
        let root = bough_core::Config::get_base_root_path(&cli.config);
        assert_eq!(root, dir.path().to_path_buf());
    }

    // bough[verify config.base-root-path+2]
    // bough[verify config.base-root-path.absolutized-relative-to-file]
    #[test]
    fn base_root_path_from_top_level_config_subdir() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let cli = parse_from_disk(
            dir.path(),
            "bough.config.toml",
            &config_toml_with_root("./src"),
        );
        let root = bough_core::Config::get_base_root_path(&cli.config);
        assert_eq!(root, dir.path().join("src"));
    }

    // bough[verify config.base-root-path.absolutized-relative-to-file]
    #[test]
    fn base_root_path_from_dotconfig_subdir_with_parent_ref() {
        let dir = tempfile::tempdir().unwrap();
        let cli = parse_from_disk(
            dir.path(),
            ".config/bough.toml",
            &config_toml_with_root(".."),
        );
        let root = bough_core::Config::get_base_root_path(&cli.config);
        assert_eq!(root, dir.path().to_path_buf());
    }

    // bough[verify config.base-root-path.absolutized-relative-to-file]
    #[test]
    fn base_root_path_from_dotconfig_subdir_relative() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        let cli = parse_from_disk(
            dir.path(),
            ".config/bough.toml",
            &config_toml_with_root("../src"),
        );
        let root = bough_core::Config::get_base_root_path(&cli.config);
        assert_eq!(root, dir.path().join("src"));
    }

    #[test]
    fn resolve_root_path_with_relative_config_dir_dot() {
        let cwd = std::env::current_dir().unwrap();
        let result = resolve_root_path(std::path::Path::new("examples/vitest-js/.config"), "..");
        assert_eq!(result, cwd.join("examples/vitest-js"));
    }

    #[test]
    fn resolve_root_path_with_relative_config_dir_subdir() {
        let cwd = std::env::current_dir().unwrap();
        let result = resolve_root_path(std::path::Path::new("some/relative/dir"), ".");
        assert_eq!(result, cwd.join("some/relative/dir"));
    }

    #[test]
    fn resolve_root_path_with_absolute_config_dir() {
        let result = resolve_root_path(std::path::Path::new("/absolute/config"), "..");
        assert_eq!(result, std::path::PathBuf::from("/absolute"));
    }

    #[test]
    fn resolve_root_path_with_absolute_root() {
        let result = resolve_root_path(std::path::Path::new("relative"), "/absolute/root");
        assert_eq!(result, std::path::PathBuf::from("/absolute/root"));
    }

    #[test]
    fn color_defaults_true_when_tty() {
        assert!(resolve_color(false, true));
    }

    #[test]
    fn color_defaults_false_when_not_tty() {
        assert!(!resolve_color(false, false));
    }

    #[test]
    fn no_color_flag_disables_color_even_with_tty() {
        assert!(!resolve_color(true, true));
    }

    #[test]
    #[serial_test::serial]
    fn no_color_env_disables_color() {
        unsafe { env::set_var("NO_COLOR", "1") };
        assert!(!resolve_color(false, true));
        unsafe { env::remove_var("NO_COLOR") };
    }

    #[test]
    #[serial_test::serial]
    fn no_color_env_disables_color_even_without_flag() {
        unsafe { env::set_var("NO_COLOR", "1") };
        assert!(!resolve_color(false, true));
        unsafe { env::remove_var("NO_COLOR") };
    }

    #[test]
    fn missing_test_cmd_fails_validation() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []
"#;
        let errors = parse_err(&["run"], toml);
        assert!(errors.iter().any(|e| matches!(e, Error::MissingTestCmd)));
    }

    #[test]
    fn test_cmd_parsed() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(bough_core::Config::get_test_cmd(&cli.config), "echo test");
    }

    #[test]
    fn test_pwd_defaults_to_base_root() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(
            bough_core::Config::get_test_pwd(&cli.config),
            std::path::PathBuf::from(".")
        );
    }

    #[test]
    fn test_pwd_uses_global_pwd() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []
pwd = "build"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_pwd(&cli.config),
            std::path::PathBuf::from("build")
        );
    }

    #[test]
    fn test_pwd_phase_overrides_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []
pwd = "build"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
pwd = "src/test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_pwd(&cli.config),
            std::path::PathBuf::from("src/test")
        );
    }

    #[test]
    fn test_env_empty_by_default() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(
            bough_core::Config::get_test_env(&cli.config),
            HashMap::new()
        );
    }

    #[test]
    fn test_env_from_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[env]
CI = "1"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_env(&cli.config),
            HashMap::from([("CI".to_string(), "1".to_string())])
        );
    }

    #[test]
    fn test_env_merges_global_and_phase() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[env]
CI = "1"
NODE_ENV = "test"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.env]
JEST_WORKERS = "4"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_env(&cli.config),
            HashMap::from([
                ("CI".to_string(), "1".to_string()),
                ("NODE_ENV".to_string(), "test".to_string()),
                ("JEST_WORKERS".to_string(), "4".to_string()),
            ])
        );
    }

    #[test]
    fn test_env_phase_empty_val_removes_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[env]
CI = "1"
NODE_ENV = "test"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.env]
NODE_ENV = ""
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_env(&cli.config),
            HashMap::from([("CI".to_string(), "1".to_string())])
        );
    }

    #[test]
    fn test_timeout_absolute_none_by_default() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            None
        );
    }

    #[test]
    fn test_timeout_absolute_from_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
absolute = 30

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            Some(chrono::Duration::seconds(30))
        );
    }

    #[test]
    fn test_timeout_absolute_phase_overrides_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
absolute = 30

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.timeout]
absolute = 60
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            Some(chrono::Duration::seconds(60))
        );
    }

    #[test]
    fn test_timeout_relative_from_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
relative = 3.0

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_relative(&cli.config),
            Some(3.0)
        );
    }

    #[test]
    fn timeout_absolute_only_is_valid() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
absolute = 30

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            Some(chrono::Duration::seconds(30))
        );
        assert_eq!(
            bough_core::Config::get_test_timeout_relative(&cli.config),
            None
        );
    }

    #[test]
    fn timeout_relative_only_is_valid() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
relative = 3.0

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            None
        );
        assert_eq!(
            bough_core::Config::get_test_timeout_relative(&cli.config),
            Some(3.0)
        );
    }

    #[test]
    fn timeout_both_is_valid() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]
absolute = 30
relative = 3.0

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            Some(chrono::Duration::seconds(30))
        );
        assert_eq!(
            bough_core::Config::get_test_timeout_relative(&cli.config),
            Some(3.0)
        );
    }

    #[test]
    fn timeout_neither_is_error() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[timeout]

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"
"#;
        parse_err(&["run"], toml);
    }

    #[test]
    fn phase_timeout_neither_is_error() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.timeout]
"#;
        parse_err(&["run"], toml);
    }

    #[test]
    fn phase_timeout_absolute_only_is_valid() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.timeout]
absolute = 60
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(&cli.config),
            Some(chrono::Duration::seconds(60))
        );
        assert_eq!(
            bough_core::Config::get_test_timeout_relative(&cli.config),
            None
        );
    }

    #[test]
    fn init_cmd_none_by_default() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(bough_core::Config::get_init_cmd(&cli.config), None);
    }

    #[test]
    fn init_cmd_parsed() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[init]
cmd = "npm install"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_init_cmd(&cli.config),
            Some("npm install".to_string())
        );
    }

    #[test]
    fn init_pwd_defaults_to_base_root() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(
            bough_core::Config::get_init_pwd(&cli.config),
            std::path::PathBuf::from(".")
        );
    }

    #[test]
    fn init_env_merges_and_removes() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[env]
CI = "1"
NODE_ENV = "test"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[init]
cmd = "npm install"

[init.env]
NODE_ENV = ""
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_init_env(&cli.config),
            HashMap::from([("CI".to_string(), "1".to_string())])
        );
    }

    #[test]
    fn reset_cmd_none_by_default() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert_eq!(bough_core::Config::get_reset_cmd(&cli.config), None);
    }

    #[test]
    fn reset_cmd_parsed() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[reset]
cmd = "npm run clean"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_reset_cmd(&cli.config),
            Some("npm run clean".to_string())
        );
    }

    #[test]
    fn reset_pwd_uses_global() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []
pwd = "build"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[reset]
cmd = "npm run clean"
"#;
        let cli = parse_ok(&["run"], toml);
        assert_eq!(
            bough_core::Config::get_reset_pwd(&cli.config),
            std::path::PathBuf::from("build")
        );
    }

    #[test]
    fn full_example_from_spec() {
        let toml = r#"
base_root_dir = "."
include = ["src/**"]
exclude = []
pwd = "build"

[timeout]
absolute = 30
relative = 3.0

[env]
CI = "1"
NODE_ENV = "test"

[lang.js]
include = ["**/*.js"]
exclude = []

[test]
cmd = "npm test"

[test.timeout]
absolute = 60

[test.env]
JEST_WORKERS = "4"

[init]
cmd = "npm install"
pwd = "setup"

[init.env]
NODE_ENV = ""

[reset]
cmd = "npm run clean"
"#;
        let cli = parse_ok(&["run"], toml);
        let c = &cli.config;

        assert_eq!(bough_core::Config::get_test_cmd(c), "npm test");
        assert_eq!(
            bough_core::Config::get_test_pwd(c),
            std::path::PathBuf::from("build")
        );
        assert_eq!(
            bough_core::Config::get_test_env(c),
            HashMap::from([
                ("CI".to_string(), "1".to_string()),
                ("NODE_ENV".to_string(), "test".to_string()),
                ("JEST_WORKERS".to_string(), "4".to_string()),
            ])
        );
        assert_eq!(
            bough_core::Config::get_test_timeout_absolute(c),
            Some(chrono::Duration::seconds(60))
        );
        assert_eq!(bough_core::Config::get_test_timeout_relative(c), Some(3.0));

        assert_eq!(
            bough_core::Config::get_init_cmd(c),
            Some("npm install".to_string())
        );
        assert_eq!(
            bough_core::Config::get_init_pwd(c),
            std::path::PathBuf::from("setup")
        );
        assert_eq!(
            bough_core::Config::get_init_env(c),
            HashMap::from([("CI".to_string(), "1".to_string())])
        );
        assert_eq!(
            bough_core::Config::get_init_timeout_absolute(c),
            Some(chrono::Duration::seconds(30))
        );
        assert_eq!(bough_core::Config::get_init_timeout_relative(c), Some(3.0));

        assert_eq!(
            bough_core::Config::get_reset_cmd(c),
            Some("npm run clean".to_string())
        );
        assert_eq!(
            bough_core::Config::get_reset_pwd(c),
            std::path::PathBuf::from("build")
        );
        assert_eq!(
            bough_core::Config::get_reset_env(c),
            HashMap::from([
                ("CI".to_string(), "1".to_string()),
                ("NODE_ENV".to_string(), "test".to_string()),
            ])
        );
        assert_eq!(
            bough_core::Config::get_reset_timeout_absolute(c),
            Some(chrono::Duration::seconds(30))
        );
        assert_eq!(bough_core::Config::get_reset_timeout_relative(c), Some(3.0));
    }
}

trait HasPhaseOverrides {
    fn phase_overrides(&self) -> &PhaseOverrides;
}

impl HasPhaseOverrides for TestPhaseConfig {
    fn phase_overrides(&self) -> &PhaseOverrides {
        &self.overrides
    }
}

impl HasPhaseOverrides for PhaseConfig {
    fn phase_overrides(&self) -> &PhaseOverrides {
        &self.overrides
    }
}

impl Config {
    fn phase_overrides<T: HasPhaseOverrides>(&self, phase: &Option<T>) -> PhaseOverrides {
        phase
            .as_ref()
            .map(|p| p.phase_overrides().clone())
            .unwrap_or_default()
    }

    fn phase_timeout_overrides(&self) -> Vec<(&str, &PhaseOverrides)> {
        let mut out = Vec::new();
        if let Some(ref t) = self.test {
            out.push(("test", &t.overrides));
        }
        if let Some(ref i) = self.init {
            out.push(("init", &i.overrides));
        }
        if let Some(ref r) = self.reset {
            out.push(("reset", &r.overrides));
        }
        out
    }
}

impl PhaseOverrides {
    fn resolve_pwd(&self, global: &PhaseOverrides) -> PathBuf {
        self.pwd
            .as_deref()
            .or(global.pwd.as_deref())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    }

    fn resolve_env(&self, global: &PhaseOverrides) -> HashMap<String, String> {
        let mut result = global.env.clone().unwrap_or_default();
        if let Some(phase_env) = &self.env {
            for (k, v) in phase_env {
                if v.is_empty() {
                    result.remove(k);
                } else {
                    result.insert(k.clone(), v.clone());
                }
            }
        }
        result
    }

    fn resolve_timeout_absolute(&self, global: &PhaseOverrides) -> Option<chrono::Duration> {
        self.timeout
            .as_ref()
            .and_then(|t| t.absolute)
            .or_else(|| global.timeout.as_ref().and_then(|t| t.absolute))
            .map(|secs| chrono::Duration::seconds(secs as i64))
    }

    fn resolve_timeout_relative(&self, global: &PhaseOverrides) -> Option<f64> {
        self.timeout
            .as_ref()
            .and_then(|t| t.relative)
            .or_else(|| global.timeout.as_ref().and_then(|t| t.relative))
    }
}

impl bough_core::Config for Config {
    fn get_workers_count(&self) -> u64 {
        self.workers
    }

    fn get_bough_state_dir(&self) -> std::path::PathBuf {
        self.get_base_root_path().join(".bough")
    }

    // bough[impl config.base-root-path+2]
    fn get_base_root_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.base_root_dir)
    }

    fn get_base_include_globs(&self) -> impl Iterator<Item = String> {
        self.include.clone().into_iter()
    }

    // bough[impl config.exclude.from-vcs-ignore]
    // bough[impl config.exclude.from-vcs-dir]
    // bough[impl config.exclude.bough-dir]
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs_ignore = collect_vcs_ignore_globs(&root);
        let vcs_dirs = collect_vcs_dir_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude
            .clone()
            .into_iter()
            .chain(vcs_ignore)
            .chain(vcs_dirs)
            .chain(std::iter::once(bough_glob))
    }

    fn get_test_cmd(&self) -> String {
        self.test
            .as_ref()
            .expect("test.cmd is required")
            .cmd
            .clone()
    }

    fn get_test_pwd(&self) -> std::path::PathBuf {
        self.phase_overrides(&self.test)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_test_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.test)
            .resolve_env(&self.phase_defaults)
    }

    fn get_test_timeout_absolute(&self) -> Option<chrono::Duration> {
        self.phase_overrides(&self.test)
            .resolve_timeout_absolute(&self.phase_defaults)
    }

    fn get_test_timeout_relative(&self) -> Option<f64> {
        self.phase_overrides(&self.test)
            .resolve_timeout_relative(&self.phase_defaults)
    }

    fn get_init_cmd(&self) -> Option<String> {
        self.init.as_ref().and_then(|i| i.cmd.clone())
    }

    fn get_init_pwd(&self) -> std::path::PathBuf {
        self.phase_overrides(&self.init)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_init_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.init)
            .resolve_env(&self.phase_defaults)
    }

    fn get_init_timeout_absolute(&self) -> Option<chrono::Duration> {
        self.phase_overrides(&self.init)
            .resolve_timeout_absolute(&self.phase_defaults)
    }

    fn get_init_timeout_relative(&self) -> Option<f64> {
        self.phase_overrides(&self.init)
            .resolve_timeout_relative(&self.phase_defaults)
    }

    fn get_reset_cmd(&self) -> Option<String> {
        self.reset.as_ref().and_then(|r| r.cmd.clone())
    }

    fn get_reset_pwd(&self) -> std::path::PathBuf {
        self.phase_overrides(&self.reset)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_reset_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.reset)
            .resolve_env(&self.phase_defaults)
    }

    fn get_reset_timeout_absolute(&self) -> Option<chrono::Duration> {
        self.phase_overrides(&self.reset)
            .resolve_timeout_absolute(&self.phase_defaults)
    }

    fn get_reset_timeout_relative(&self) -> Option<f64> {
        self.phase_overrides(&self.reset)
            .resolve_timeout_relative(&self.phase_defaults)
    }

    fn get_find_number(&self) -> usize {
        self.find.number
    }

    fn get_find_number_per_file(&self) -> usize {
        self.find.number_per_file
    }

    fn get_find_factors(&self) -> Vec<bough_core::Factor> {
        self.find.factors.clone()
    }

    fn get_langs(&self) -> impl Iterator<Item = bough_core::LanguageId> {
        self.lang.keys().copied().collect::<Vec<_>>().into_iter()
    }

    // bough[impl config.lang.include.derived]
    fn get_lang_include_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        self.lang
            .get(&language_id)
            .map(|c| c.include.clone())
            .unwrap_or_default()
            .into_iter()
    }

    // bough[impl config.lang.exclude.from-vcs-ignore]
    // bough[impl config.lang.exclude.from-vcs-dir]
    // bough[impl config.lang.exclude.bough-dir]
    // bough[impl config.lang.exclude.derived]
    fn get_lang_exclude_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs_ignore = collect_vcs_ignore_globs(&root);
        let vcs_dirs = collect_vcs_dir_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude
            .iter()
            .cloned()
            .chain(vcs_ignore)
            .chain(vcs_dirs)
            .chain(std::iter::once(bough_glob))
            .chain(
                self.lang
                    .get(&language_id)
                    .map(|c| c.exclude.clone())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn get_lang_skip_queries(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        self.lang
            .get(&language_id)
            .and_then(|c| c.skip.as_ref())
            .map(|s| s.query.clone())
            .unwrap_or_default()
            .into_iter()
    }
}

impl crate::render::Render for Config {
    fn markdown(&self) -> String {
        format!(
            "{t}# Bough Config{r}\n\n```json\n{}\n```",
            facet_json::to_string(self).unwrap(),
            t = crate::render::TITLE,
            r = crate::render::RESET,
        )
    }

    fn terse(&self) -> String {
        facet_json::to_string(self).unwrap()
    }

    fn verbose(&self) -> String {
        format!(
            "{t}Bough Config{r}\n\n{}",
            facet_json::to_string(self).unwrap(),
            t = crate::render::TITLE,
            r = crate::render::RESET,
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod config_render_tests {
    use super::*;
    use crate::render::{TITLE, RESET, Render};

    fn fixture() -> Config {
        let json = r#"{"base_root_dir":".","include":[],"exclude":[],"lang":{},"find":{"number":1,"number_per_file":1,"factors":[]}}"#;
        facet_json::from_str::<Config>(json).unwrap()
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# Bough Config\n\n```json\n"));
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(!out.contains('\n'));
        assert!(out.starts_with('{'));
    }

    #[test]
    fn verbose() {
        let plain = fixture().verbose().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("Bough Config\n\n"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('{'));
        assert!(out.contains("base_root_dir"));
    }
}

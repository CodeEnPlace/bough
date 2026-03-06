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

    #[facet(args::named, args::short = 'f', default = "terse")]
    pub format: String,

    #[facet(args::subcommand)]
    pub command: Command,

    #[facet(args::config, args::env_prefix = "BOUGH")]
    pub config: Config,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Terse,
    Verbose,
    Markdown,
    Json,
}

#[derive(Facet, Debug)]
#[repr(u8)]
pub enum Command {
    Show {
        #[facet(args::subcommand)]
        what: ShowCommand,
    },
    Run,
    Noop,
}

#[derive(Facet, Debug)]
#[repr(u8)]
pub enum ShowCommand {
    Files {
        #[facet(args::positional, default)]
        lang: Option<bough_core::LanguageId>,
    },
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

    #[error("unknown format: {0}")]
    #[diagnostic(
        code(bough::config::unknown_format),
        help("valid formats: terse, verbose, markdown, json")
    )]
    UnknownFormat(String),

    #[error("{0}")]
    #[diagnostic(code(bough::config::parse))]
    Parse(String),
}

impl Cli {
    pub fn format(&self) -> Result<Format, Error> {
        match self.format.as_str() {
            "terse" => Ok(Format::Terse),
            "verbose" => Ok(Format::Verbose),
            "markdown" => Ok(Format::Markdown),
            "json" => Ok(Format::Json),
            other => Err(Error::UnknownFormat(other.to_string())),
        }
    }

    pub fn validate(&self) -> Vec<Error> {
        let mut errors = Vec::new();
        if let Err(e) = self.format() {
            errors.push(e);
        }
        // cli[impl config.include.at-least-one]
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

// cli[impl config.exclude.from-vcs-ignore]
// cli[impl config.lang.exclude.from-vcs-ignore]
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

// cli[impl config.base-root-path]
// cli[impl config.base-root-path.sub]
// cli[impl config.base-root-path.parent]
// cli[impl config.base-root-path.parent.sub]
pub fn resolve_config_from(start: &std::path::Path) -> Option<(std::path::PathBuf, String)> {
    let result = find_config_candidates_from(start)
        .into_iter()
        .find(|(_, p)| std::path::Path::new(p).is_file());
    match &result {
        Some((root, path)) => info!(path, root = %root.display(), "resolved config file"),
        None => warn!("no config file found"),
    }
    result
}

pub fn resolve_config() -> Option<(std::path::PathBuf, String)> {
    env::current_dir().ok().and_then(|d| resolve_config_from(&d))
}

pub fn resolve_config_path() -> Option<String> {
    resolve_config().map(|(_, path)| path)
}

pub fn resolve_config_root() -> Option<std::path::PathBuf> {
    resolve_config().map(|(root, _)| root)
}

pub fn parse() -> Cli {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .env(|env| env)
        .file(|f| {
            f.default_paths(find_config_candidates().into_iter().map(|(_, p)| p).collect::<Vec<_>>())
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
    fn show_files_subcommand() {
        let cli = parse_ok(&["show", "files"], MINIMAL_TOML);
        assert!(matches!(
            cli.command,
            Command::Show {
                what: ShowCommand::Files { .. }
            }
        ));
    }

    #[test]
    fn run_subcommand() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        assert!(matches!(cli.command, Command::Run));
    }

    // cli[verify config.include.at-least-one]
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

    // cli[verify config.lang.include.derived]
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

    // cli[verify config.lang.exclude.derived]
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
        assert!(globs.iter().position(|g| g == "target/**") < globs.iter().position(|g| g == "node_modules/**"));
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
        let yaml = "include:\n  - \"src/**\"\nexclude: []\nlang:\n  js:\n    include:\n      - \"**/*.js\"\n    exclude: []\n";
        let cli = try_parse_from(&["run"], Some((yaml, "config.yaml"))).expect("should parse");
        assert_eq!(cli.config.include, vec!["src/**"]);
    }

    // cli[verify config.base-root-path]
    #[test]
    fn resolve_config_finds_root_for_top_level_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("bough.config.toml"), MINIMAL_TOML).unwrap();
        let (root, _) = resolve_config_from(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    // cli[verify config.base-root-path.sub]
    #[test]
    fn resolve_config_finds_root_for_sub_dir_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".config")).unwrap();
        std::fs::write(dir.path().join(".config/bough.toml"), MINIMAL_TOML).unwrap();
        let (root, _) = resolve_config_from(dir.path()).unwrap();
        assert_eq!(root, dir.path());
    }

    // cli[verify config.base-root-path.parent]
    #[test]
    fn resolve_config_finds_root_in_parent() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("subproject");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::write(dir.path().join("bough.config.toml"), MINIMAL_TOML).unwrap();
        let (root, _) = resolve_config_from(&child).unwrap();
        assert_eq!(root, dir.path());
    }

    // cli[verify config.base-root-path.parent.sub]
    #[test]
    fn resolve_config_finds_root_in_parent_sub_dir() {
        let dir = tempfile::tempdir().unwrap();
        let child = dir.path().join("subproject");
        std::fs::create_dir_all(&child).unwrap();
        std::fs::create_dir_all(dir.path().join(".config")).unwrap();
        std::fs::write(dir.path().join(".config/bough.toml"), MINIMAL_TOML).unwrap();
        let (root, _) = resolve_config_from(&child).unwrap();
        assert_eq!(root, dir.path());
    }

    // cli[verify config.exclude.bough-dir]
    #[test]
    fn base_exclude_globs_includes_bough_dir() {
        let cli = parse_ok(&["run"], MINIMAL_TOML);
        let globs: Vec<String> = bough_core::Config::get_base_exclude_globs(&cli.config).collect();
        assert!(globs.iter().any(|g| g.contains(".bough")));
    }

    // cli[verify config.lang.exclude.bough-dir]
    // cli[verify config.lang.exclude.from-vcs-ignore]
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

    // cli[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_reads_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "node_modules\n*.log\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.contains(&"**/node_modules".to_string()));
        assert!(globs.contains(&"**/*.log".to_string()));
    }

    // cli[verify config.exclude.from-vcs-ignore]
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

    // cli[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_skips_negation_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "dist\n!dist/keep\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert_eq!(globs, vec!["**/dist"]);
    }

    // cli[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_handles_slash_prefixed_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "/build\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert_eq!(globs, vec!["build".to_string()]);
    }

    // cli[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_preserves_glob_patterns() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(".gitignore"), "**/*.o\nsrc/**/*.tmp\n").unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.contains(&"**/*.o".to_string()));
        assert!(globs.contains(&"src/**/*.tmp".to_string()));
    }

    // cli[verify config.exclude.from-vcs-ignore]
    #[test]
    fn collect_vcs_ignore_returns_empty_when_no_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let globs = collect_vcs_ignore_globs(dir.path());
        assert!(globs.is_empty());
    }

    // cli[verify config.exclude.from-vcs-ignore]
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
}

impl bough_core::Config for Config {
    fn get_workers_count(&self) -> u64 {
        self.workers
    }

    fn get_bough_state_dir(&self) -> std::path::PathBuf {
        self.get_base_root_path().join(".bough")
    }

    fn get_base_root_path(&self) -> std::path::PathBuf {
        resolve_config_root()
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| ".".into()))
    }

    fn get_base_include_globs(&self) -> impl Iterator<Item = String> {
        self.include.clone().into_iter()
    }

    // cli[impl config.exclude.from-vcs-ignore]
    // cli[impl config.exclude.bough-dir]
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs = collect_vcs_ignore_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude.clone().into_iter().chain(vcs).chain(std::iter::once(bough_glob))
    }

    fn get_langs(&self) -> impl Iterator<Item = bough_core::LanguageId> {
        self.lang.keys().copied().collect::<Vec<_>>().into_iter()
    }

    // cli[impl config.lang.include.derived]
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

    // cli[impl config.lang.exclude.from-vcs-ignore]
    // cli[impl config.lang.exclude.bough-dir]
    // cli[impl config.lang.exclude.derived]
    fn get_lang_exclude_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs = collect_vcs_ignore_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude
            .iter()
            .cloned()
            .chain(vcs)
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
}

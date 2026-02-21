use std::path::PathBuf;

use pollard_core::config::{Commands, Config, LanguageId, Ordering, Vcs};
use serde::Serialize;

use crate::feedback::{DiffStyle, Style};
use crate::Cli;

// All fields are resolved and non-optional. No defaults are applied here;
// every value must be provided by CLI args or config. No Option types allowed.
#[derive(Serialize)]
pub struct Session {
    pub language: LanguageId,
    pub vcs: Vcs,
    pub working_dir: PathBuf,
    pub parallelism: usize,
    pub report_dir: PathBuf,
    pub ordering: Ordering,
    pub sub_dir: PathBuf,
    pub files: String,
    pub ignore_mutants: Vec<String>,
    pub timeout_absolute: u64,
    pub timeout_relative: f64,
    pub style: Style,
    pub diff: DiffStyle,
    pub no_color: bool,
    pub force_on_dirty_repo: bool,
    pub config_path: PathBuf,
    pub commands: Commands,
}

macro_rules! require {
    ($missing:ident, $expr:expr, $name:literal, $hint:literal) => {
        {
            let val = $expr;
            if val.is_none() {
                $missing.push(concat!($name, " (", $hint, ")"));
            }
            val
        }
    };
}

impl Session {
    pub fn from_cli_and_config(
        cli: &Cli,
        config: Config,
        config_path: PathBuf,
    ) -> Result<Self, String> {
        let mut missing: Vec<&str> = Vec::new();

        let language = require!(missing, cli.language.or(config.language), "language", "use -l/--language or set in config");
        let vcs = require!(missing, cli.vcs.or(config.vcs), "vcs", "use --vcs or set in config");
        let working_dir = require!(missing, cli.working_dir.clone().or(config.working_dir.clone()), "working_dir", "use --working-dir or set in config");
        let parallelism = require!(missing, cli.parallelism.or(config.parallelism), "parallelism", "use --parallelism or set in config");
        let report_dir = require!(missing, cli.report_dir.clone().or(config.report_dir.clone()), "report_dir", "use --report-dir or set in config");
        let ordering = require!(missing, cli.ordering.or(config.ordering), "ordering", "use --ordering or set in config");
        let files = require!(missing, cli.files.clone().or(config.files.clone()), "files", "use --files or set in config");
        let timeout_absolute = require!(missing, cli.timeout_absolute.or(config.timeout.absolute), "timeout.absolute", "use --timeout-absolute or set in config");
        let timeout_relative = require!(missing, cli.timeout_relative.or(config.timeout.relative), "timeout.relative", "use --timeout-relative or set in config");

        if !missing.is_empty() {
            return Err(format!(
                "missing required settings:\n  - {}",
                missing.join("\n  - ")
            ));
        }

        Ok(Self {
            language: language.unwrap(),
            vcs: vcs.unwrap(),
            working_dir: working_dir.unwrap(),
            parallelism: parallelism.unwrap(),
            report_dir: report_dir.unwrap(),
            ordering: ordering.unwrap(),
            sub_dir: cli.sub_dir.clone()
                .or(config.sub_dir.clone())
                .unwrap_or_else(|| PathBuf::from(".")),
            files: files.unwrap(),
            timeout_absolute: timeout_absolute.unwrap(),
            timeout_relative: timeout_relative.unwrap(),
            ignore_mutants: if cli.ignore_mutants.is_empty() {
                config.ignore_mutants
            } else {
                cli.ignore_mutants.clone()
            },
            style: cli.style.clone(),
            diff: cli.diff.clone(),
            no_color: cli.no_color,
            force_on_dirty_repo: cli.force_on_dirty_repo,
            config_path,
            commands: config.commands,
        })
    }
}

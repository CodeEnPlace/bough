mod action;
mod feedback;

use action::Action;
use clap::{Parser, Subcommand};
use feedback::{ApplyRecord, MutationRecord, RenderOutput, Style};
use log::LevelFilter;
use pollard_core::config::{Config, LanguageId};
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::{
    Hash, Language, MutatedFile, MutationKind, SourceFile, find_mutation_points,
    generate_mutation_substitutions,
};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, global = true, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long, global = true)]
    language: Option<LanguageId>,

    #[arg(short, long, default_value = "plain", global = true)]
    style: Style,

    #[arg(long, default_value = "unified", global = true)]
    diff: feedback::DiffStyle,

    #[arg(long, env = "NO_COLOR", hide = true, default_value_t = false, global = true)]
    no_color: bool,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true, default_value_t = false)]
    force_on_dirty_repo: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Mutate {
        #[command(subcommand)]
        action: MutateAction,
    },
}

#[derive(Subcommand)]
enum MutateAction {
    Generate {
        #[arg(short, long)]
        file: String,
    },
    View {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        hash: Hash,
    },
    Apply {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        hash: Hash,
    },
}

fn log_level(cli: &Cli) -> LevelFilter {
    match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

fn generate_for_language<L: Language>(file: &SourceFile) -> Vec<MutationRecord>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    let mut records = Vec::new();
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(point) {
            records.push(MutationRecord {
                source_path: file.path().to_owned(),
                source_hash: file.hash().clone(),
                mutated_hash: mutated.hash().clone(),
                kind: point.kind.into(),
                start_line: point.span.start.line,
                start_char: point.span.start.char,
                end_line: point.span.end.line,
                end_char: point.span.end.char,
                original: file.content()[point.span.start.byte..point.span.end.byte].to_string(),
                replacement,
            });
        }
    }
    records
}

fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    glob::glob(pattern)
        .unwrap_or_else(|e| {
            eprintln!("invalid glob pattern: {e}");
            std::process::exit(1);
        })
        .filter_map(|entry| match entry {
            Ok(path) if path.is_file() => Some(path),
            Ok(_) => None,
            Err(e) => {
                eprintln!("glob error: {e}");
                None
            }
        })
        .collect()
}

fn generate(language: &LanguageId, pattern: &str) -> Vec<MutationRecord> {
    let paths = expand_glob(pattern);
    let mut records = Vec::new();
    for path in &paths {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_records = match language {
            LanguageId::Javascript => generate_for_language::<JavaScript>(&file),
            LanguageId::Typescript => generate_for_language::<TypeScript>(&file),
        };
        records.append(&mut file_records);
    }
    records
}

fn find_mutated_by_hash<'a, L: Language>(
    file: &'a SourceFile,
    target: &Hash,
) -> Option<MutatedFile<'a>> {
    let points = find_mutation_points::<L>(file);
    for point in &points {
        for (_, mutated) in generate_mutation_substitutions::<L>(point) {
            if mutated.hash() == target {
                return Some(mutated);
            }
        }
    }
    None
}

fn view(
    language: &LanguageId,
    input: &Path,
    hash: &Hash,
    diff_style: feedback::DiffStyle,
) -> feedback::DiffRecord {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageId::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageId::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    });

    feedback::DiffRecord {
        old: file.content().to_string(),
        new: mutated.content().to_string(),
        path: file.path().display().to_string(),
        diff_style,
    }
}

fn apply(language: &LanguageId, input: &Path, hash: &Hash) -> (Vec<Action>, ApplyRecord) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageId::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageId::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    });

    let actions = vec![Action::WriteFile {
        path: input.to_owned(),
        content: mutated.content().to_string(),
    }];

    let record = ApplyRecord {
        source_path: file.path().to_owned(),
        source_hash: file.hash().clone(),
        mutated_hash: mutated.hash().clone(),
    };

    (actions, record)
}



fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .parse_default_env()
        .filter_level(log_level(&cli))
        .init();

    let cwd = std::env::current_dir().expect("failed to get current directory");
    let discovered = match &cli.config {
        Some(path) => Some((path.clone(), Config::read(path))),
        None => Config::discover(&cwd),
    };
    let (config_path, config) = match discovered {
        Some((path, Ok(config))) => {
            log::info!("loaded config from {}", path.display());
            log::debug!("config: {}", serde_json::to_string(&config).expect("failed to serialize config"));
            (path, config)
        }
        Some((path, Err(e))) => {
            eprintln!("error in {}: {e}", path.display());
            std::process::exit(1);
        }
        None => {
            eprintln!("no config file found (searched from {})", cwd.display());
            std::process::exit(1);
        }
    };
    let _ = &config_path;

    let language = cli.language.or(config.language).unwrap_or_else(|| {
        eprintln!("no language specified (use -l/--language or set language in config)");
        std::process::exit(1);
    });

    let (actions, renders): (Vec<Action>, Vec<Box<dyn RenderOutput>>) = match &cli.command {
        Command::Mutate { action } => match action {
            MutateAction::Generate { file: pattern } => {
                let renders: Vec<Box<dyn RenderOutput>> = generate(&language, pattern)
                    .into_iter()
                    .map(|r| Box::new(r) as Box<dyn RenderOutput>)
                    .collect();
                (vec![], renders)
            }
            MutateAction::View { file: input, hash } => {
                let record = view(&language, input, hash, cli.diff.clone());
                (vec![], vec![Box::new(record)])
            }
            MutateAction::Apply { file: input, hash } => {
                let (actions, record) = apply(&language, input, hash);
                (actions, vec![Box::new(record)])
            }
        },
    };

    if !actions.is_empty() && !cli.force_on_dirty_repo && action::repo_is_dirty() {
        eprintln!("repo has uncommitted changes, use --force-on-dirty-repo to proceed");
        std::process::exit(1);
    }

    for action in actions {
        action.apply().expect("failed to apply action");
    }

    for render in &renders {
        render.render(&cli.style, cli.no_color);
    }
}

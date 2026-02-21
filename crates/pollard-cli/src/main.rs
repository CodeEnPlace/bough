mod feedback;

use clap::{Parser, Subcommand, ValueEnum};
use feedback::{ApplyRecord, MutationRecord, RenderOutput, Style};
use log::LevelFilter;
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
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long)]
    language: LanguageArg,

    #[arg(short, long, default_value = "plain", global = true)]
    style: Style,

    #[arg(long, default_value = "unified", global = true)]
    diff: feedback::DiffStyle,

    #[arg(long, env = "NO_COLOR", hide = true, default_value_t = false)]
    no_color: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, ValueEnum)]
enum LanguageArg {
    #[value(alias = "js")]
    Javascript,
    #[value(alias = "ts")]
    Typescript,
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

fn generate(language: &LanguageArg, pattern: &str) -> Vec<MutationRecord> {
    let paths = expand_glob(pattern);
    let mut records = Vec::new();
    for path in &paths {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_records = match language {
            LanguageArg::Javascript => generate_for_language::<JavaScript>(&file),
            LanguageArg::Typescript => generate_for_language::<TypeScript>(&file),
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
    language: &LanguageArg,
    input: &Path,
    hash: &Hash,
    diff_style: feedback::DiffStyle,
) -> feedback::DiffRecord {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageArg::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageArg::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
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

fn apply(language: &LanguageArg, input: &Path, hash: &Hash) -> ApplyRecord {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageArg::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageArg::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    });

    std::fs::write(input, mutated.content()).expect("failed to write mutated file");

    ApplyRecord {
        source_path: file.path().to_owned(),
        source_hash: file.hash().clone(),
        mutated_hash: mutated.hash().clone(),
    }
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(log_level(&cli))
        .parse_default_env()
        .init();

    match &cli.command {
        Command::Mutate { action } => match action {
            MutateAction::Generate { file: pattern } => {
                for record in generate(&cli.language, pattern) {
                    record.render(&cli.style, cli.no_color);
                }
            }
            MutateAction::View { file: input, hash } => {
                view(&cli.language, input, hash, cli.diff.clone()).render(&cli.style, cli.no_color);
            }
            MutateAction::Apply { file: input, hash } => {
                apply(&cli.language, input, hash).render(&cli.style, cli.no_color);
            }
        },
    }
}

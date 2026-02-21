use clap::{Parser, Subcommand, ValueEnum};
use log::LevelFilter;
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::{Language, MutatedFile, MutationKind, SourceFile, Span, find_mutation_points, generate_mutation_substitutions};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long, help = "Suppress all log output")]
    quiet: bool,

    #[arg(short, long)]
    language: LanguageArg,

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
        input: PathBuf,
    },
    Apply {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn log_level(cli: &Cli) -> LevelFilter {
    if cli.quiet {
        return LevelFilter::Off;
    }
    match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

#[derive(Serialize)]
struct MutationRecord<'a> {
    mutated_file: &'a MutatedFile<'a>,
    kind: MutationKind,
    span: Span,
    replacement: &'a str,
}

fn generate_for_language<L: Language>(file: &SourceFile)
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    for point in &points {
        for (replacement, mutated) in &generate_mutation_substitutions::<L>(point) {
            let record = MutationRecord {
                mutated_file: mutated,
                kind: point.kind.into(),
                span: point.span.clone(),
                replacement,
            };
            println!("{}", serde_json::to_string(&record).expect("failed to serialize"));
        }
    }
}

fn generate(language: &LanguageArg, input: &PathBuf) {
    let file = SourceFile::read(input).expect("failed to read input file");
    match language {
        LanguageArg::Javascript => generate_for_language::<JavaScript>(&file),
        LanguageArg::Typescript => generate_for_language::<TypeScript>(&file),
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
            MutateAction::Generate { input } => {
                generate(&cli.language, &input);
            }
            MutateAction::Apply { input } => {
                log::info!("applying mutation from {}", input.display());
            }
        },
    }
}

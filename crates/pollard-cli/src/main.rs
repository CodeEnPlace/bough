mod io;
mod mutate;
mod session;
mod steps;

use clap::{Parser, Subcommand};
use io::{Action, Report, Style};
use pollard_core::Hash;
use pollard_core::config::{Config, LanguageId, Ordering, Vcs};
use session::Session;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, global = true)]
    language: Option<LanguageId>,

    #[arg(short, long, default_value = "plain", global = true)]
    style: Style,

    #[arg(long, default_value = "unified", global = true)]
    diff: io::DiffStyle,

    #[arg(
        long,
        env = "NO_COLOR",
        hide = true,
        default_value_t = false,
        global = true
    )]
    no_color: bool,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true)]
    vcs: Option<Vcs>,

    #[arg(long, global = true)]
    working_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    parallelism: Option<usize>,

    #[arg(long, global = true)]
    report_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    ordering: Option<Ordering>,

    #[arg(long, global = true)]
    sub_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    files: Option<String>,

    #[arg(long, global = true)]
    ignore_mutants: Vec<String>,

    #[arg(long, global = true)]
    timeout_absolute: Option<u64>,

    #[arg(long, global = true)]
    timeout_relative: Option<f64>,

    #[arg(long, global = true, default_value_t = false)]
    exec: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Mutate {
        #[command(subcommand)]
        action: MutateAction,
    },
    Step {
        #[command(subcommand)]
        action: StepAction,
    },
}

#[derive(Debug, Subcommand)]
enum StepAction {
    FindFiles,
    DeriveMutants,
    CreateWorkspaces,
    ApplyMutantToWorkspace {
        #[arg(short, long)]
        workspace: String,
        #[arg(long)]
        mutant: Hash,
    },
    SetupWorkspace {
        #[arg(short, long)]
        workspace: String,
    },
    TestWorkspace {
        #[arg(short, long)]
        workspace: String,
    },
    ResetWorkspace {
        #[arg(short, long)]
        workspace: String,
        #[arg(short, long)]
        rev: String,
    },
    Cleanup,
}

#[derive(Debug, Subcommand)]
enum MutateAction {
    Generate {
        #[arg(short, long)]
        file: String,
    },
    View {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        mutant: Hash,
    },
    Apply {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        mutant: Hash,
    },
}

fn render_and_apply(actions: Vec<Action>, reports: Vec<Box<dyn Report>>, session: &Session) {
    for report in &reports {
        report.render(&session.style, session.no_color, 0);
    }

    if actions.is_empty() {
        return;
    }

    if !session.exec {
        eprintln!("{} action(s) pending, pass --exec to apply", actions.len());
        return;
    }

    for action in actions {
        action.render(&session.style, session.no_color, 0);
        action.apply().expect("failed to apply action");
    }
}

fn main() {
    let cli = Cli::parse();

    let cwd = std::env::current_dir().expect("failed to get current directory");
    let discovered = match &cli.config {
        Some(path) => Some((path.clone(), Config::read(path))),
        None => Config::discover(&cwd),
    };
    let (config_path, config) = match discovered {
        Some((path, Ok(config))) => (path, config),
        Some((path, Err(e))) => {
            eprintln!("error in {}: {e}", path.display());
            std::process::exit(1);
        }
        None => {
            eprintln!("no config file found (searched from {})", cwd.display());
            std::process::exit(1);
        }
    };
    let session = Session::from_cli_and_config(&cli, config, config_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    let (actions, reports): (Vec<Action>, Vec<Box<dyn Report>>) = match &cli.command {
        Command::Mutate {
            action: MutateAction::Generate { file: pattern },
        } => {
            let (actions, reports) = mutate::generate::run(&session.language, pattern);
            (
                actions,
                reports
                    .into_iter()
                    .map(|r| Box::new(r) as Box<dyn Report>)
                    .collect(),
            )
        }
        Command::Mutate {
            action:
                MutateAction::View {
                    file: input,
                    mutant: hash,
                },
        } => {
            let (actions, report) =
                mutate::view::run(&session.language, input, hash, session.diff.clone());
            (actions, vec![Box::new(report)])
        }
        Command::Mutate {
            action:
                MutateAction::Apply {
                    file: input,
                    mutant: hash,
                },
        } => {
            let (actions, report) = mutate::apply::run(&session.language, input, hash);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action: StepAction::FindFiles,
        } => {
            let (actions, report) = steps::find_files::run(&session);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action: StepAction::DeriveMutants,
        } => {
            let find_report = steps::find_files::run(&session).1;
            let (actions, report) = steps::derive_mutants::run(&session, &find_report.files);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action: StepAction::CreateWorkspaces,
        } => {
            let (actions, report) = steps::create_workspaces::run(&session);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action:
                StepAction::ApplyMutantToWorkspace {
                    workspace,
                    mutant: hash,
                },
        } => {
            let (actions, report) =
                steps::apply_mutant_to_workspace::run(&session, workspace, hash);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action: StepAction::SetupWorkspace { workspace },
        } => {
            let (actions, report) = steps::setup_workspace::run(&session, workspace);
            (
                actions,
                report
                    .into_iter()
                    .map(|r| Box::new(r) as Box<dyn Report>)
                    .collect(),
            )
        }
        Command::Step {
            action: StepAction::TestWorkspace { workspace },
        } => {
            let (actions, report) = steps::test_workspace::run(&session, workspace);
            (
                actions,
                report
                    .into_iter()
                    .map(|r| Box::new(r) as Box<dyn Report>)
                    .collect(),
            )
        }
        Command::Step {
            action: StepAction::ResetWorkspace { workspace, rev },
        } => {
            let (actions, report) = steps::reset_workspace::run(&session, workspace, rev);
            (actions, vec![Box::new(report)])
        }
        Command::Step {
            action: StepAction::Cleanup,
        } => {
            let (actions, report) = steps::cleanup::run(&session);
            (actions, vec![Box::new(report)])
        }
    };

    render_and_apply(actions, reports, &session);
}

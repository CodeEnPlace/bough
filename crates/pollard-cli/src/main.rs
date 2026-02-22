mod io;
mod mutate;
mod steps;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use io::{Action, Report};
use pollard_core::Hash;
use pollard_session::{PartialSession, Session, SessionSkipped, discover_config, read_config};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(flatten)]
    settings: PartialSession,

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
    DumpSession,
    Completions {
        #[arg(value_enum)]
        shell: Shell,
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
        for action in actions {
            action.render(&session.style, session.no_color, 0);
        }
        return;
    }

    for action in actions {
        action.render(&session.style, session.no_color, 0);
        action.apply().expect("failed to apply action");
    }
}

fn build_session(cli: Cli) -> (Session, Command) {
    let cwd = std::env::current_dir().expect("failed to get current directory");
    let discovered = match &cli.config {
        Some(path) => Some((path.clone(), read_config(path))),
        None => discover_config(&cwd),
    };
    let (config_path, config_partial) = match discovered {
        Some((path, Ok(partial))) => (path, partial),
        Some((path, Err(e))) => {
            eprintln!("error in {}: {e}", path.display());
            std::process::exit(1);
        }
        None => {
            eprintln!("no config file found (searched from {})", cwd.display());
            std::process::exit(1);
        }
    };

    let merged = cli.settings.merge(config_partial);

    let mut session = merged
        .resolve(SessionSkipped { config_path })
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1);
        });

    session.normalize_paths();

    (session, cli.command)
}

fn main() {
    let cli = Cli::parse();

    if let Command::Completions { shell } = &cli.command {
        clap_complete::generate(*shell, &mut Cli::command(), "pollard", &mut std::io::stdout());
        return;
    }

    let (session, command) = build_session(cli);

    let (actions, reports): (Vec<Action>, Vec<Box<dyn Report>>) = match &command {
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
        Command::DumpSession => {
            let report = io::SessionReport::new(&session);
            (vec![], vec![Box::new(report)])
        }
        Command::Completions { .. } => unreachable!(),
    };

    render_and_apply(actions, reports, &session);
}

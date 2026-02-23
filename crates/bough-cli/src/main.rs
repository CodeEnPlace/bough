mod io;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use io::Render;
use bough_session::{PartialSession, Session, SessionSkipped, discover_config, read_config};
use std::io::IsTerminal;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bough", about = "Cross-language mutation testing")]
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
    DumpSession,
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn render_and_apply(actions: Vec<io::Action>, reports: Vec<Box<dyn Render>>, session: &Session) {
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

    if !std::io::stdout().is_terminal() {
        session.no_color = true;
    }

    (session, cli.command)
}

fn main() {
    let cli = Cli::parse();

    if let Command::Completions { shell } = &cli.command {
        clap_complete::generate(
            *shell,
            &mut Cli::command(),
            "bough",
            &mut std::io::stdout(),
        );
        return;
    }

    let (session, command) = build_session(cli);

    let (actions, reports): (Vec<io::Action>, Vec<Box<dyn Render>>) = match &command {
        Command::DumpSession => {
            let report = io::SessionReport::new(&session);
            (vec![], vec![Box::new(report)])
        }
        Command::Completions { .. } => unreachable!(),
    };

    render_and_apply(actions, reports, &session);
}

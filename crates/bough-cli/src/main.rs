mod config;
mod phase_runner;
mod render;
mod steps;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use render::{Render, Style};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bough", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(long = "config-file", global = true)]
    config_file: Option<PathBuf>,

    #[arg(long = "config-override", global = true)]
    config_overrides: Vec<PathBuf>,

    #[arg(long = "config", global = true)]
    configs: Vec<String>,

    #[arg(long, global = true, default_value = "terse")]
    output_style: Style,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Show {
        #[command(subcommand)]
        subject: ShowSubject,
    },
    Workspace {
        #[command(subcommand)]
        action: WorkspaceAction,
    },
    Mutate {
        #[arg()]
        workspace: String,
        #[arg()]
        mutation_hash: String,
    },
    Run,
    Clean,
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

#[derive(Debug, Subcommand)]
enum WorkspaceAction {
    Make,
    List,
    Init {
        #[arg()]
        name: String,
    },
    Reset {
        #[arg()]
        name: String,
    },
    Test {
        #[arg()]
        name: String,
        #[arg()]
        mutation_hash: String,
    },
    Drop {
        #[arg()]
        name: String,
    },
}

#[derive(Debug, Subcommand)]
enum ShowSubject {
    Config,
    Src,
    Mutations,
    Mutation {
        #[arg()]
        hash: String,
    },
    TestIds,
}

fn main() {
    let cli = Cli::parse();

    if let Command::Completions { shell } = &cli.command {
        clap_complete::generate(*shell, &mut Cli::command(), "bough", &mut std::io::stdout());
        return;
    }

    let cfg = config::load(&cli).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });

    match &cli.command {

        Command::Workspace { action } => match action {
            WorkspaceAction::Make => {
                let result = steps::make_workspace::run(&cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            WorkspaceAction::List => {
                let result = steps::list_workspaces::run(&cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            WorkspaceAction::Init { name } => {
                let ws = bough_core::WorkspaceId::new(name, &cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let path = PathBuf::from(cfg.working_dir()).join(&*ws);
                let result = steps::init_workspace::run(&cfg, &path).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            WorkspaceAction::Reset { name } => {
                let ws = bough_core::WorkspaceId::new(name, &cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let path = PathBuf::from(cfg.working_dir()).join(&*ws);
                let result = steps::reset_workspace::run(&cfg, &path).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            WorkspaceAction::Test {
                name,
                mutation_hash,
            } => {
                let ws = bough_core::WorkspaceId::new(name, &cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let path = PathBuf::from(cfg.working_dir()).join(&*ws);
                let result =
                    steps::test_workspace::run(&cfg, &path, mutation_hash).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            WorkspaceAction::Drop { name } => {
                let ws = bough_core::WorkspaceId::new(name, &cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let result = steps::drop_workspace::run(&cfg, &ws).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
        },

        Command::Show { subject } => match subject {
            ShowSubject::Config => {
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                cfg.render(&cli.output_style, no_color, 0);
            }
            ShowSubject::Src => {
                let result = steps::get_src_files::run(&cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            ShowSubject::Mutations => {
                let src_files = steps::get_src_files::run(&cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let result = steps::get_mutations::run(&src_files, &cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            ShowSubject::Mutation { hash } => {
                let result =
                    steps::get_mutation_result::run(&cfg, hash).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
            ShowSubject::TestIds => {
                let result = steps::get_all_test_ids::run(&cfg).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
                let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
                result.render(&cli.output_style, no_color, 0);
            }
        },

        Command::Mutate {
            workspace,
            mutation_hash,
        } => {
            let ws = bough_core::WorkspaceId::new(workspace, &cfg).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let path = PathBuf::from(cfg.working_dir()).join(&*ws);
            let result =
                steps::mutate_workspace::run(&cfg, &path, mutation_hash).unwrap_or_else(|e| {
                    eprintln!("{e}");
                    std::process::exit(1);
                });
            let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
            result.render(&cli.output_style, no_color, 0);
        }

        Command::Run => {
            let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());

            let ((src_files, mutations), test_ids) = std::thread::scope(|s| {
                let mutations_handle = s.spawn(|| {
                    let src_files = steps::get_src_files::run(&cfg).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    });
                    let mutations =
                        steps::get_mutations::run(&src_files, &cfg).unwrap_or_else(|e| {
                            eprintln!("{e}");
                            std::process::exit(1);
                        });
                    (src_files, mutations)
                });

                let test_ids_handle = s.spawn(|| {
                    steps::get_all_test_ids::run(&cfg).unwrap_or_else(|e| {
                        eprintln!("{e}");
                        std::process::exit(1);
                    })
                });

                let mutations = mutations_handle.join().expect("mutations thread panicked");
                let test_ids = test_ids_handle.join().expect("test_ids thread panicked");
                (mutations, test_ids)
            });

            src_files.render(&cli.output_style, no_color, 0);
            mutations.render(&cli.output_style, no_color, 0);
            test_ids.render(&cli.output_style, no_color, 0);
        }

        Command::Clean => {
            let result = steps::clean::run(&cfg).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
            result.render(&cli.output_style, no_color, 0);
        }

        Command::Completions { .. } => unreachable!(),
    }
}

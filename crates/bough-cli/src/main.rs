mod config;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "bough", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long = "config-override", global = true)]
    config_overrides: Vec<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Command::Completions { shell } => {
            clap_complete::generate(
                *shell,
                &mut Cli::command(),
                "bough",
                &mut std::io::stdout(),
            );
        }
    }
}

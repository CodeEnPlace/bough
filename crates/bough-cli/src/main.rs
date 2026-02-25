mod config;
mod render;

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
    DumpConfig,
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
        Command::DumpConfig => {
            let cfg = config::load(&cli).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
            let no_color = !std::io::IsTerminal::is_terminal(&std::io::stdout());
            cfg.render(&cli.output_style, no_color, 0);
        }
    }
}

mod config;

use config::{Command, ShowCommand, parse, resolve_config_path};
use tracing::Level;

fn main() {
    let cli = parse();

    let log_level = match cli.verbose {
        0 => Level::ERROR,
        1 => Level::WARN,
        2 => Level::INFO,
        3 => Level::DEBUG,
        _ => Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let config_path = resolve_config_path();

    match cli.command {
        Command::Show { ref what } => match what {
            ShowCommand::Cli => {
                if let Some(ref path) = config_path {
                    println!("config file: {path}");
                }
                println!("{cli:#?}");
            }
            ShowCommand::Config => {
                if let Some(ref path) = config_path {
                    println!("config file: {path}");
                }
                println!("workers: {}", cli.config.workers);
            }
            ShowCommand::File => match config_path {
                Some(ref path) => println!("{path}"),
                None => println!("no config file found"),
            },
        },
        Command::Run => {}
    }
}

mod config;

use bough_core::Session;
use config::{Command, ShowCommand, parse, resolve_config_path};
use tracing::{Level, debug, info};

fn main() {
    let cli = parse();

    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    let config_path = resolve_config_path();

    info!(log_level = %log_level, "tracing initialized");

    let session = Session::new(cli.config.clone());

    match cli.command {
        Command::Show { ref what } => {
            debug!(subcommand = ?what, "executing show command");
            match what {
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
            }
        }
        Command::Run => {
            info!("starting run");
        }
    }
}

mod config;
mod render;

use bough_core::{File, Session};
use config::{Command, ShowCommand, parse, resolve_config_path};
use render::{Noop, Render};
use tracing::{Level, debug, info};

use crate::render::BaseFiles;

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

    let session = Session::new(cli.config.clone()).expect("session creation");

    info!("session initalized");

    let result: Box<dyn Render> = match cli.command {
        Command::Show { ref what } => {
            debug!(subcommand = ?what, "executing show command");
            match what {
                ShowCommand::Files => {
                    let base = session.base();
                    let twigs = base.twigs().collect::<Vec<_>>();
                    let files = twigs
                        .iter()
                        .map(|twig| File::new(base, &twig))
                        .collect::<Vec<_>>();

                    let paths = files.iter().map(|file| file.resolve()).collect();

                    Box::new(BaseFiles(paths))
                }
            }
        }
        Command::Run => {
            info!("starting run");
            Box::new(Noop)
        }
        Command::Noop => {
            info!("starting run");
            Box::new(Noop)
        }
    };

    println!("{}", result.render(cli.format().expect("valid format")));
}

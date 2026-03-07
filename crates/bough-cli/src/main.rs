mod config;
mod render;

use bough_core::{File, Session};
use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, info};

use crate::render::{BaseFiles, MutantFiles};

fn main() {
    let cli = parse();

    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    tracing_subscriber::fmt().with_max_level(log_level).init();

    info!(log_level = %log_level, "tracing initialized");

    let result: Box<dyn Render> = match cli.command {
        Command::Show { ref show } => {
            debug!(subcommand = ?show, "executing show command");
            match show {
                Show::Config => Box::new(cli.config.clone()),

                Show::Files { lang: None } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let base = session.base();
                    let twigs = base.twigs().collect::<Vec<_>>();
                    let files = twigs
                        .iter()
                        .map(|twig| File::new(base, &twig))
                        .collect::<Vec<_>>();

                    let paths = files.iter().map(|file| file.resolve()).collect();

                    Box::new(BaseFiles(paths))
                }

                Show::Files { lang: Some(lang) } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let base = session.base();
                    let twigs = base.mutant_twigs().collect::<Vec<_>>();
                    let files = twigs
                        .iter()
                        .filter(|(l, _)| l == lang)
                        .map(|(_, twig)| File::new(base, &twig))
                        .collect::<Vec<_>>();

                    let paths = files.iter().map(|file| file.resolve()).collect();

                    Box::new(MutantFiles(*lang, paths))
                }

                Show::Mutations {
                    lang: None,
                    file: _,
                } => todo!(),

                Show::Mutations {
                    lang: Some(lang),
                    file: None,
                } => todo!(),

                Show::Mutations {
                    lang: Some(lang),
                    file: Some(file),
                } => todo!(),

                Show::Mutation { hash } => todo!(),
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

    println!("{}", result.render(cli.format));
}

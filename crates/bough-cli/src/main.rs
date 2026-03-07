mod config;
mod render;

use bough_core::{File, Session};
use bough_typed_hash::TypedHashable;
use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, info};

use crate::render::{AllMutations, BaseFiles, FileMutations, LangMutations, MutantFiles, SingleMutation, find_mutation_by_hash};

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
                } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan");
                    Box::new(AllMutations(mutations))
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: None,
                } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan")
                        .into_iter()
                        .filter(|m| m.mutant().lang() == *lang)
                        .collect();
                    Box::new(LangMutations(*lang, mutations))
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: Some(file),
                } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan")
                        .into_iter()
                        .filter(|m| m.mutant().lang() == *lang && m.mutant().twig().path() == file.as_path())
                        .collect();
                    Box::new(FileMutations(*lang, file.clone(), mutations))
                }

                Show::Mutation { hash } => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    session.tend_add_missing_states().expect("tend states");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan");
                    let mutation = find_mutation_by_hash(hash, mutations);
                    let lang = mutation.mutant().lang();
                    let file_path = bough_core::File::new(base, mutation.mutant().twig()).resolve();
                    let file_src = std::fs::read_to_string(&file_path).expect("read source file");
                    let (before, ctx_span) = mutation.mutant()
                        .get_contextual_fragment(base, 3)
                        .expect("context fragment");
                    let mutated_src = mutation.apply_to_complete_src_string(&file_src);
                    let original_len = mutation.mutant().span().end().byte() - mutation.mutant().span().start().byte();
                    let subst_len = mutation.subst().len();
                    let end_byte = if subst_len >= original_len {
                        ctx_span.end().byte() + (subst_len - original_len)
                    } else {
                        ctx_span.end().byte() - (original_len - subst_len)
                    };
                    let after = &mutated_src[ctx_span.start().byte()..end_byte];
                    let mutation_hash = mutation.hash().expect("hashing should not fail");
                    let state = session.get_state().get(&mutation_hash)
                        .expect("state not found for mutation");
                    Box::new(SingleMutation { state, before, after: after.to_string(), lang })
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

    println!("{}", result.render(&cli));
}

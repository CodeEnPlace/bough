mod config;
mod render;

use std::sync::{Arc, Mutex};

use bough_core::{File, Mutation, Session, State};
use bough_typed_hash::TypedHashable;
use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, info};

use crate::render::{
    AllMutations, BaseFiles, FileMutations, LangMutations, MutantFiles, SingleMutation,
    find_mutation_by_hash,
};

fn collect_states(session: &Session<config::Config>, mutations: Vec<Mutation>) -> Vec<State> {
    mutations
        .into_iter()
        .map(|m| {
            let hash = m.hash().expect("hash");
            session
                .get_state()
                .get(&hash)
                .expect("state not found for mutation")
        })
        .collect()
}

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
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    session.tend_add_missing_states().expect("tend states");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan");
                    let states = collect_states(&session, mutations);
                    Box::new(AllMutations(states))
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: None,
                } => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    session.tend_add_missing_states().expect("tend states");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan")
                        .into_iter()
                        .filter(|m| m.mutant().lang() == *lang)
                        .collect();
                    let states = collect_states(&session, mutations);
                    Box::new(LangMutations(*lang, states))
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: Some(file),
                } => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    session.tend_add_missing_states().expect("tend states");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan")
                        .into_iter()
                        .filter(|m| {
                            m.mutant().lang() == *lang && m.mutant().twig().path() == file.as_path()
                        })
                        .collect();
                    let states = collect_states(&session, mutations);
                    Box::new(FileMutations(*lang, file.clone(), states))
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
                    let (before, ctx_span) = mutation
                        .mutant()
                        .get_contextual_fragment(base, 3)
                        .expect("context fragment");
                    let mutated_src = mutation.apply_to_complete_src_string(&file_src);
                    let original_len = mutation.mutant().span().end().byte()
                        - mutation.mutant().span().start().byte();
                    let subst_len = mutation.subst().len();
                    let end_byte = if subst_len >= original_len {
                        ctx_span.end().byte() + (subst_len - original_len)
                    } else {
                        ctx_span.end().byte() - (original_len - subst_len)
                    };
                    let after = &mutated_src[ctx_span.start().byte()..end_byte];
                    let mutation_hash = mutation.hash().expect("hashing should not fail");
                    let state = session
                        .get_state()
                        .get(&mutation_hash)
                        .expect("state not found for mutation");
                    Box::new(SingleMutation {
                        state,
                        before,
                        after: after.to_string(),
                        lang,
                    })
                }
            }
        }

        Command::Step { ref step } => {
            debug!(subcommand = ?step, "executing step command");

            match step {
                config::Step::TendState => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    let added = session
                        .tend_add_missing_states()
                        .expect("tend add missing states");
                    let removed = session
                        .tend_remove_stale_states()
                        .expect("tend remove stale states");
                    Box::new(render::TendState { added, removed })
                }

                config::Step::TendWorkspaces => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    let workers = cli.config.workers as usize;
                    let workspace_ids = session.tend_workspaces(workers).expect("tend workspaces");
                    Box::new(render::TendWorkspaces { workspace_ids })
                }

                config::Step::InitWorkspace { workspace_id } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let wid =
                        bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = session.bind_workspace(&wid).expect("bind workspace");
                    let outcome = workspace
                        .run_init(&cli.config, None)
                        .expect("init workspace");
                    Box::new(render::InitWorkspace {
                        workspace_id: wid,
                        outcome,
                    })
                }

                config::Step::ResetWorkspace { workspace_id } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let wid =
                        bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = session.bind_workspace(&wid).expect("bind workspace");
                    let outcome = workspace
                        .run_reset(&cli.config, None)
                        .expect("reset workspace");
                    Box::new(render::ResetWorkspace {
                        workspace_id: wid,
                        outcome,
                    })
                }

                config::Step::ApplyMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let wid =
                        bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan");
                    let mutation = render::find_mutation_by_hash(mutation_hash, mutations);
                    let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
                    workspace.write_mutant(&mutation).expect("apply mutation");
                    let hash_str = mutation.hash().expect("hash").to_string();
                    Box::new(render::ApplyMutation {
                        workspace_id: wid,
                        mutation_hash: hash_str,
                    })
                }

                config::Step::UnapplyMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let session = Session::new(cli.config.clone()).expect("session creation");
                    let wid =
                        bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
                    workspace.revert_mutant().expect("unapply mutation");
                    Box::new(render::UnapplyMutation {
                        workspace_id: wid,
                        mutation_hash: mutation_hash.clone(),
                    })
                }

                config::Step::TestMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let mut session = Session::new(cli.config.clone()).expect("session creation");
                    let wid =
                        bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let base = session.base();
                    let mutations: Vec<_> = base
                        .mutations()
                        .collect::<Result<Vec<_>, _>>()
                        .expect("mutation scan");
                    let mutation = render::find_mutation_by_hash(mutation_hash, mutations);
                    let hash_str = mutation.hash().expect("hash").to_string();
                    let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
                    workspace.write_mutant(&mutation).expect("apply mutation");
                    let outcome = workspace
                        .run_test(&cli.config, None)
                        .expect("test mutation");
                    workspace.revert_mutant().expect("revert mutation");
                    let status = if outcome.exit_code() != 0 {
                        bough_core::Status::Caught
                    } else {
                        bough_core::Status::Missed
                    };
                    let status_str = if outcome.exit_code() != 0 {
                        "caught"
                    } else {
                        "missed"
                    };
                    session.set_state(&mutation, status).expect("set state");
                    Box::new(render::TestMutation {
                        workspace_id: wid,
                        mutation_hash: hash_str,
                        status: status_str,
                        duration: outcome.duration(),
                    })
                }
            }
        }

        Command::Run => {
            info!("starting run");
            let mut session = Session::new(cli.config.clone()).expect("session creation");
            let added = session
                .tend_add_missing_states()
                .expect("tend add missing states");
            let removed = session
                .tend_remove_stale_states()
                .expect("tend remove stale states");

            println!("{}", (render::TendState { added, removed }).render(&cli));

            let workers = cli.config.workers as usize;
            let workspace_ids = session.tend_workspaces(workers).expect("tend workspaces");

            println!(
                "{}",
                (render::TendWorkspaces {
                    workspace_ids: workspace_ids.clone(),
                })
                .render(&cli)
            );

            let session = Arc::new(Mutex::new(session));

            std::thread::scope(|scope| {
                for workspace_id in workspace_ids {
                    let session = Arc::clone(&session);
                    let cli = cli.clone();

                    scope.spawn(move || {
                        let mut workspace = session
                            .lock()
                            .unwrap()
                            .bind_workspace(&workspace_id)
                            .expect("bind workspace");

                        if let Ok(outcome) = workspace.run_init(&cli.config, None) {
                            println!(
                                "{}",
                                render::InitWorkspace {
                                    workspace_id: workspace_id.clone(),
                                    outcome,
                                }
                                .render(&cli)
                            );
                        }

                        loop {
                            let hash_to_test =
                                session.lock().unwrap().get_next_mutation_needing_test();

                            if let Some(hash_to_test) = hash_to_test {
                                let mutation_state = session
                                    .lock()
                                    .unwrap()
                                    .get_state()
                                    .get(&hash_to_test)
                                    .unwrap();

                                let mutation = mutation_state.mutation();

                                if let Ok(outcome) = workspace.run_reset(&cli.config, None) {
                                    println!(
                                        "{}",
                                        render::ResetWorkspace {
                                            workspace_id: workspace_id.clone(),
                                            outcome,
                                        }
                                        .render(&cli)
                                    );
                                }

                                workspace.write_mutant(&mutation).expect("apply mutation");
                                let outcome = workspace
                                    .run_test(&cli.config, None)
                                    .expect("test mutation");
                                workspace.revert_mutant().expect("revert mutation");
                                let status = if outcome.exit_code() != 0 {
                                    bough_core::Status::Caught
                                } else {
                                    bough_core::Status::Missed
                                };

                                let status_str = if outcome.exit_code() != 0 {
                                    "caught"
                                } else {
                                    "missed"
                                };

                                session
                                    .lock()
                                    .unwrap()
                                    .set_state(&mutation, status)
                                    .expect("set state");

                                println!(
                                    "{}",
                                    (render::TestMutation {
                                        workspace_id: workspace_id.clone(),
                                        mutation_hash: format!("{}", hash_to_test),
                                        status: status_str,
                                        duration: outcome.duration(),
                                    })
                                    .render(&cli)
                                );
                            } else {
                                break;
                            }
                        }
                    });
                }
            });

            Box::new(Noop)
        }

        Command::Noop => {
            info!("starting run");
            Box::new(Noop)
        }
    };

    println!("{}", result.render(&cli));
}

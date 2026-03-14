mod config;
mod render;
mod show_all_files;
mod show_all_mutations;
mod show_file_mutations;
mod show_language_files;
mod show_language_mutations;
mod show_single_mutation;
mod step_tend_state;
mod step_tend_workspaces;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bough_core::Session;
use bough_typed_hash::TypedHashable;
use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, error, info, warn};


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
                    show_all_files::ShowAllFiles::run(cli.config.clone())
                }

                Show::Files { lang: Some(lang) } => {
                    show_language_files::ShowLanguageFiles::run(cli.config.clone(), *lang)
                }

                Show::Mutations {
                    lang: None,
                    file: _,
                } => {
                    show_all_mutations::ShowAllMutations::run(cli.config.clone())
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: None,
                } => {
                    show_language_mutations::ShowLanguageMutations::run(cli.config.clone(), *lang)
                }

                Show::Mutations {
                    lang: Some(lang),
                    file: Some(file),
                } => {
                    show_file_mutations::ShowFileMutations::run(cli.config.clone(), *lang, file.clone())
                }

                Show::Mutation { hash } => {
                    show_single_mutation::ShowSingleMutation::run(cli.config.clone(), hash)
                }
            }
        }

        Command::Step { ref step } => {
            debug!(subcommand = ?step, "executing step command");

            match step {
                config::Step::TendState => {
                    step_tend_state::StepTendState::run(cli.config.clone())
                }

                config::Step::TendWorkspaces => {
                    step_tend_workspaces::StepTendWorkspaces::run(cli.config.clone())
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

            let base = session.base();

            let init_duration = match base.run_init(&cli.config, None) {
                Ok(outcome) => {
                    if outcome.exit_code() != 0 {
                        eprintln!("base init failed (exit {})", outcome.exit_code());
                        std::process::exit(1);
                    }
                    Some(outcome.duration())
                }
                Err(bough_core::PhaseError::NoCmdConfigured) => None,
                Err(e) => {
                    eprintln!("base init error: {e}");
                    std::process::exit(1);
                }
            };

            let reset_duration = match base.run_reset(&cli.config, None) {
                Ok(outcome) => {
                    if outcome.exit_code() != 0 {
                        eprintln!("base reset failed (exit {})", outcome.exit_code());
                        std::process::exit(1);
                    }
                    Some(outcome.duration())
                }
                Err(bough_core::PhaseError::NoCmdConfigured) => None,
                Err(e) => {
                    eprintln!("base reset error: {e}");
                    std::process::exit(1);
                }
            };

            let test_outcome = base
                .run_test(&cli.config, None)
                .expect("base test execution");
            if test_outcome.exit_code() != 0 {
                eprintln!("base test failed (exit {})", test_outcome.exit_code());
                std::process::exit(1);
            }

            let test_duration = test_outcome.duration();
            let benchmark = render::BenchmarkTimesInBase {
                init: init_duration,
                reset: reset_duration,
                test: test_duration,
            };
            println!("{}", benchmark.render(&cli));

            let total = session.get_count_mutation_needing_test() as u64;
            let session = Arc::new(Mutex::new(session));
            let done = Arc::new(AtomicBool::new(false));

            let pb = if cli.progress {
                let pb = indicatif::ProgressBar::new(total);
                pb.set_style(
                    indicatif::ProgressStyle::with_template(
                        "{wide_bar:.cyan/blue} {pos}/{len} [{elapsed_precise} elapsed, {eta_precise} remaining]",
                    )
                    .unwrap()
                    .progress_chars("=+ "),
                );
                pb.enable_steady_tick(std::time::Duration::from_millis(100));
                Some(pb)
            } else {
                None
            };

            #[deny(clippy::unwrap_used, clippy::expect_used)]
            let _: () = std::thread::scope(|scope| {
                if let Some(ref pb) = pb {
                    let pb = pb.clone();
                    let session = Arc::clone(&session);
                    let done = Arc::clone(&done);
                    scope.spawn(move || {
                        while !done.load(Ordering::Relaxed) {
                            let Ok(mut guard) = session.lock() else {
                                break;
                            };
                            let remaining = guard.get_count_mutation_needing_test() as u64;
                            drop(guard);
                            pb.set_position(total.saturating_sub(remaining));
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        pb.set_position(total);
                        pb.finish();
                    });
                }

                for workspace_id in workspace_ids {
                    let session = Arc::clone(&session);
                    let cli = cli.clone();
                    let done = done.clone();

                    scope.spawn(move || {
                        let Ok(guard) = session.lock() else {
                            error!(%workspace_id, "mutex poisoned binding workspace");
                            return;
                        };
                        let Ok(mut workspace) = guard.bind_workspace(&workspace_id) else {
                            error!(%workspace_id, "failed to bind workspace");
                            return;
                        };
                        drop(guard);

                        if let Ok(outcome) = workspace.run_init(&cli.config, init_duration) {
                            if !cli.progress {
                                println!(
                                    "{}",
                                    render::InitWorkspace {
                                        workspace_id: workspace_id.clone(),
                                        outcome,
                                    }
                                    .render(&cli)
                                );
                            }
                        }

                        loop {
                            let Ok(mut guard) = session.lock() else {
                                error!(%workspace_id, "mutex poisoned in worker");
                                break;
                            };
                            let hash_to_test = guard.get_next_mutation_needing_test();
                            drop(guard);

                            let Some(hash_to_test) = hash_to_test else {
                                break;
                            };

                            let Ok(guard) = session.lock() else {
                                error!(%workspace_id, "mutex poisoned getting state");
                                break;
                            };
                            let Some(mutation_state) = guard.get_state().get(&hash_to_test) else {
                                warn!(%workspace_id, %hash_to_test, "state not found for mutation");
                                continue;
                            };
                            let mutation = mutation_state.mutation();
                            drop(guard);

                            if let Ok(outcome) =
                                workspace.run_reset(&cli.config, reset_duration)
                            {
                                if !cli.progress {
                                    println!(
                                        "{}",
                                        render::ResetWorkspace {
                                            workspace_id: workspace_id.clone(),
                                            outcome,
                                        }
                                        .render(&cli)
                                    );
                                }
                            }

                            if let Err(e) = workspace.write_mutant(&mutation) {
                                error!(%workspace_id, err = %e, "failed to apply mutation");
                                continue;
                            }
                            let outcome = match workspace
                                .run_test(&cli.config, Some(test_duration))
                            {
                                Ok(o) => o,
                                Err(e) => {
                                    error!(%workspace_id, err = %e, "test execution failed");
                                    let _ = workspace.revert_mutant();
                                    continue;
                                }
                            };
                            if let Err(e) = workspace.revert_mutant() {
                                error!(%workspace_id, err = %e, "failed to revert mutation");
                                break;
                            }

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

                            let Ok(mut guard) = session.lock() else {
                                error!(%workspace_id, "mutex poisoned setting state");
                                break;
                            };
                            if let Err(e) = guard.set_state(&mutation, status) {
                                error!(%workspace_id, err = ?e, "failed to set state");
                            }
                            drop(guard);

                            if !cli.progress {
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
                            }
                        }

                        done.store(true, Ordering::Relaxed);
                    });
                }
            });

            Box::new(Noop)
        }

        Command::Find { ref lang, ref file } => {
            let session = Session::new(cli.config.clone()).expect("session creation");
            let results = session.find_best_mutations().expect("find best mutations");
            let filtered: Vec<_> = results
                .into_iter()
                .filter(|(_, state, _)| {
                    if let Some(l) = lang {
                        if state.mutation().mutant().lang() != *l {
                            return false;
                        }
                    }
                    if let Some(f) = file {
                        if state.mutation().mutant().twig().path() != f.as_path() {
                            return false;
                        }
                    }
                    true
                })
                .collect();
            Box::new(render::FindBestMutations(filtered))
        }

        Command::Noop => {
            info!("starting run");
            Box::new(Noop)
        }
    };

    println!("{}", result.render(&cli));
}

mod config;
mod find_best_mutations;
mod render;
mod show_all_files;
mod show_all_mutations;
mod show_file_mutations;
mod show_language_files;
mod show_language_mutations;
mod show_single_mutation;
mod step_apply_mutation;
mod step_init_workspace;
mod step_reset_workspace;
mod step_tend_state;
mod step_tend_workspaces;
mod step_test_mutation;
mod step_unapply_mutation;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bough_core::Session;
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};

use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, error, info, warn};

fn resolve_mutation_hash(session: &Session<config::Config>, hash: &str) -> bough_core::Mutation {
    let mutations: Vec<_> = session.base().mutations().collect::<Result<Vec<_>, _>>().expect("mutation scan");
    let unvalidated = UnvalidatedHash::new(hash.to_string());
    let hashes: Vec<_> = mutations.iter().map(|m| m.hash().expect("hash")).collect();
    let matched = unvalidated.validate(&hashes).expect("hash resolution failed");
    let matched_bytes = matched.as_bytes();
    mutations.into_iter().find(|m| m.hash().unwrap().as_bytes() == matched_bytes).unwrap()
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

    let session = Session::new(cli.config.clone()).expect("session creation");
    let session = Arc::new(Mutex::new(session));

    let result: Box<dyn Render> = match cli.command {
        Command::Show { ref show } => {
            debug!(subcommand = ?show, "executing show command");
            match show {
                Show::Config => Box::new(cli.config.clone()),

                Show::Files { lang: None } => show_all_files::ShowAllFiles::run(session.lock().unwrap()),

                Show::Files { lang: Some(lang) } => {
                    show_language_files::ShowLanguageFiles::run(session.lock().unwrap(), *lang)
                }

                Show::Mutations {
                    lang: None,
                    file: _,
                } => show_all_mutations::ShowAllMutations::run(session.lock().unwrap()),

                Show::Mutations {
                    lang: Some(lang),
                    file: None,
                } => show_language_mutations::ShowLanguageMutations::run(session.lock().unwrap(), *lang),

                Show::Mutations {
                    lang: Some(lang),
                    file: Some(file),
                } => show_file_mutations::ShowFileMutations::run(
                    session.lock().unwrap(),
                    *lang,
                    file.clone(),
                ),

                Show::Mutation { hash } => {
                    show_single_mutation::ShowSingleMutation::run(session.lock().unwrap(), hash)
                }
            }
        }

        Command::Step { ref step } => {
            debug!(subcommand = ?step, "executing step command");

            match step {
                config::Step::TendState => step_tend_state::StepTendState::run(session.lock().unwrap()),

                config::Step::TendWorkspaces => {
                    step_tend_workspaces::StepTendWorkspaces::run(session.lock().unwrap(), &cli.config)
                }

                config::Step::InitWorkspace { workspace_id } => {
                    let guard = session.lock().unwrap();
                    let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_init_workspace::StepInitWorkspace::run(&workspace, &cli.config, None).expect("init workspace")
                }

                config::Step::ResetWorkspace { workspace_id } => {
                    let guard = session.lock().unwrap();
                    let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_reset_workspace::StepResetWorkspace::run(&workspace, &cli.config, None).expect("reset workspace")
                }

                config::Step::ApplyMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let guard = session.lock().unwrap();
                    let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let mutation = resolve_mutation_hash(&guard, mutation_hash);
                    let mut workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_apply_mutation::StepApplyMutation::run(&mut workspace, &mutation).expect("apply mutation")
                }

                config::Step::UnapplyMutation {
                    workspace_id,
                    mutation_hash,
                } => step_unapply_mutation::StepUnapplyMutation::run(
                    session.lock().unwrap(),
                    workspace_id,
                    mutation_hash,
                ),

                config::Step::TestMutation {
                    workspace_id,
                    mutation_hash,
                } => step_test_mutation::StepTestMutation::run(
                    session.lock().unwrap(),
                    &cli.config,
                    workspace_id,
                    mutation_hash,
                ),
            }
        }

        Command::Run => {
            info!("starting run");
            let tend_state = step_tend_state::StepTendState::run(session.lock().unwrap());
            println!("{}", tend_state.render(&cli));

            let tend_workspaces = step_tend_workspaces::StepTendWorkspaces::run(session.lock().unwrap(), &cli.config);
            println!("{}", tend_workspaces.render(&cli));
            let workspace_ids = tend_workspaces.workspace_ids;

            let (init_duration, reset_duration, test_duration, total) = {
                let mut guard = session.lock().unwrap();
                let base = guard.base();

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

                let total = guard.get_count_mutation_needing_test() as u64;
                (init_duration, reset_duration, test_outcome.duration(), total)
            };

            let benchmark = render::BenchmarkTimesInBase {
                init: init_duration,
                reset: reset_duration,
                test: test_duration,
            };
            println!("{}", benchmark.render(&cli));

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

                        if let Ok(init) = step_init_workspace::StepInitWorkspace::run(&workspace, &cli.config, init_duration) {
                            if !cli.progress {
                                println!("{}", init.render(&cli));
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

                            if let Ok(reset) = step_reset_workspace::StepResetWorkspace::run(&workspace, &cli.config, reset_duration) {
                                if !cli.progress {
                                    println!("{}", reset.render(&cli));
                                }
                            }

                            if let Err(e) = step_apply_mutation::StepApplyMutation::run(&mut workspace, &mutation) {
                                error!(%workspace_id, err = %e, "failed to apply mutation");
                                continue;
                            }
                            let outcome = match workspace.run_test(&cli.config, Some(test_duration))
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
            find_best_mutations::FindBestMutations::run(session.lock().unwrap(), *lang, file.clone())
        }

        Command::Noop => {
            info!("starting run");
            Box::new(Noop)
        }
    };

    println!("{}", result.render(&cli));
}

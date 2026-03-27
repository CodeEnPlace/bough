use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bough_core::Session;

use crate::config::{Cli, Config};
use crate::render::{Noop, Render};
use crate::{
    step_apply_mutation, step_init_workspace, step_reset_workspace, step_tend_state,
    step_tend_workspaces, step_test_mutation, step_unapply_mutation,
};
use tracing::{error, info, warn};

pub struct Run;

impl Run {
    pub fn run(session: Arc<Mutex<Session<Config>>>, cli: &Cli) -> Box<dyn Render> {
        info!("starting run");
        let tend_state = step_tend_state::StepTendState::run(session.lock().unwrap());
        println!("{}", tend_state.render(cli));

        let tend_workspaces =
            step_tend_workspaces::StepTendWorkspaces::run(session.lock().unwrap(), &cli.config);
        println!("{}", tend_workspaces.render(cli));
        let workspace_ids = tend_workspaces.workspace_ids;

        let (init_duration, reset_duration, test_duration, total) = {
            let mut guard = session.lock().unwrap();
            let base = guard.base();

            let init_duration = match base.run_init(&cli.config, None) {
                Ok(bough_core::PhaseOutcome::Completed { exit_code, duration, .. }) => {
                    if exit_code != 0 {
                        eprintln!("base init failed (exit {exit_code})");
                        std::process::exit(1);
                    }
                    Some(chrono::Duration::from_std(duration).expect("duration overflow"))
                }
                Ok(bough_core::PhaseOutcome::TimedOut { .. }) => {
                    eprintln!("base init timed out");
                    std::process::exit(1);
                }
                Err(bough_core::PhaseError::NoCmdConfigured) => None,
                Err(e) => {
                    eprintln!("base init error: {e}");
                    std::process::exit(1);
                }
            };

            let reset_duration = match base.run_reset(&cli.config, None) {
                Ok(bough_core::PhaseOutcome::Completed { exit_code, duration, .. }) => {
                    if exit_code != 0 {
                        eprintln!("base reset failed (exit {exit_code})");
                        std::process::exit(1);
                    }
                    Some(chrono::Duration::from_std(duration).expect("duration overflow"))
                }
                Ok(bough_core::PhaseOutcome::TimedOut { .. }) => {
                    eprintln!("base reset timed out");
                    std::process::exit(1);
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
            match &test_outcome {
                bough_core::PhaseOutcome::Completed { exit_code, .. } if *exit_code != 0 => {
                    eprintln!("base test failed (exit {exit_code})");
                    std::process::exit(1);
                }
                bough_core::PhaseOutcome::TimedOut { .. } => {
                    eprintln!("base test timed out");
                    std::process::exit(1);
                }
                _ => {}
            }

            let total = guard.get_count_mutation_needing_test() as u64;
            (
                init_duration,
                reset_duration,
                chrono::Duration::from_std(test_outcome.duration()).expect("duration overflow"),
                total,
            )
        };

        let benchmark = crate::render::BenchmarkTimesInBase {
            init: init_duration.map(|d| d.to_std().expect("duration overflow")),
            reset: reset_duration.map(|d| d.to_std().expect("duration overflow")),
            test: test_duration.to_std().expect("duration overflow"),
        };
        println!("{}", benchmark.render(cli));

        let done = Arc::new(AtomicBool::new(false));

        let interactive = std::io::stdout().is_terminal();
        let pb = if interactive {
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

                    if let Ok(init) = step_init_workspace::StepInitWorkspace::run(
                        &workspace,
                        &cli.config,
                        init_duration,
                    ) {
                        if !interactive {
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
                            warn!(
                                %workspace_id,
                                %hash_to_test,
                                "state not found for mutation"
                            );
                            continue;
                        };
                        let mutation = mutation_state.mutation();
                        drop(guard);

                        if let Ok(reset) = step_reset_workspace::StepResetWorkspace::run(
                            &workspace,
                            &cli.config,
                            reset_duration,
                        ) {
                            if !interactive {
                                println!("{}", reset.render(&cli));
                            }
                        }

                        if let Err(e) =
                            step_apply_mutation::StepApplyMutation::run(&mut workspace, &mutation)
                        {
                            error!(%workspace_id, err = %e, "failed to apply mutation");
                            continue;
                        }
                        let test_result = match step_test_mutation::StepTestMutation::run(
                            &workspace,
                            &cli.config,
                            &mutation,
                            Some(test_duration),
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                error!(%workspace_id, err = %e, "test execution failed");
                                let _ = step_unapply_mutation::StepUnapplyMutation::run(
                                    &mut workspace,
                                );
                                continue;
                            }
                        };
                        if let Err(e) =
                            step_unapply_mutation::StepUnapplyMutation::run(&mut workspace)
                        {
                            error!(%workspace_id, err = %e, "failed to revert mutation");
                            break;
                        }

                        let Ok(mut guard) = session.lock() else {
                            error!(%workspace_id, "mutex poisoned setting state");
                            break;
                        };
                        if let Err(e) =
                            guard.set_state(&mutation, test_result.status_value.clone())
                        {
                            error!(%workspace_id, err = ?e, "failed to set state");
                        }
                        drop(guard);

                        if !interactive {
                            println!("{}", test_result.render(&cli));
                        }
                    }

                    done.store(true, Ordering::Relaxed);
                });
            }
        });

        Box::new(Noop)
    }
}

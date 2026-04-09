use std::io::IsTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use bough_lib::Session;

use crate::config::{Cli, Config};
use crate::render::{Noop, Render};
use crate::{
    step_apply_mutation, step_init_workspace, step_reset_workspace, step_tend_state,
    step_tend_workspaces, step_test_mutation, step_unapply_mutation,
};
use tracing::{error, info, warn};

pub struct Run;

#[derive(Default)]
struct EmaEta {
    last_pos: u64,
    last_elapsed: f64,
    rate: f64,
}

impl EmaEta {
    /// Smoothing factor in [0, 1]; lower = smoother. 0.3 ≈ window of ~6 samples.
    const ALPHA: f64 = 0.3;

    fn update(&mut self, pos: u64, elapsed: f64) -> f64 {
        let dp = pos.saturating_sub(self.last_pos);
        let dt = elapsed - self.last_elapsed;
        if dp > 0 && dt > 0.0 {
            let inst = dp as f64 / dt;
            self.rate = if self.rate == 0.0 {
                inst
            } else {
                Self::ALPHA * inst + (1.0 - Self::ALPHA) * self.rate
            };
            self.last_pos = pos;
            self.last_elapsed = elapsed;
        }
        self.rate
    }
}

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

            let init_duration = match bough_lib::run_init_in_base(base, &cli.config, None) {
                Ok(bough_lib::PhaseOutcome::Completed {
                    exit_code,
                    duration,
                    ..
                }) => {
                    if exit_code != 0 {
                        eprintln!("base init failed (exit {exit_code})");
                        std::process::exit(1);
                    }
                    Some(chrono::Duration::from_std(duration).expect("duration overflow"))
                }
                Ok(bough_lib::PhaseOutcome::TimedOut { .. }) => {
                    eprintln!("base init timed out");
                    std::process::exit(1);
                }
                Err(bough_lib::PhaseError::NoCmdConfigured) => None,
                Err(e) => {
                    eprintln!("base init error: {e}");
                    std::process::exit(1);
                }
            };

            let reset_duration = match bough_lib::run_reset_in_base(base, &cli.config, None) {
                Ok(bough_lib::PhaseOutcome::Completed {
                    exit_code,
                    duration,
                    ..
                }) => {
                    if exit_code != 0 {
                        eprintln!("base reset failed (exit {exit_code})");
                        std::process::exit(1);
                    }
                    Some(chrono::Duration::from_std(duration).expect("duration overflow"))
                }
                Ok(bough_lib::PhaseOutcome::TimedOut { .. }) => {
                    eprintln!("base reset timed out");
                    std::process::exit(1);
                }
                Err(bough_lib::PhaseError::NoCmdConfigured) => None,
                Err(e) => {
                    eprintln!("base reset error: {e}");
                    std::process::exit(1);
                }
            };

            let test_outcome = bough_lib::run_test_in_base(base, &cli.config, None)
                .expect("base test execution");
            match &test_outcome {
                bough_lib::PhaseOutcome::Completed { exit_code, .. } if *exit_code != 0 => {
                    eprintln!("base test failed (exit {exit_code})");
                    std::process::exit(1);
                }
                bough_lib::PhaseOutcome::TimedOut { .. } => {
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

        // https://github.com/CodeEnPlace/bough/issues/40 — escalating signal handling
        let done_for_handler = done.clone();
        ctrlc::set_handler(move || {
            done_for_handler.store(true, Ordering::SeqCst);
            eprintln!("\nInterrupted. Waiting for in-flight tests to finish...");
            eprintln!(
                "Repeated Ctrl-C will NOT force-kill — this avoids orphaning test processes."
            );
            eprintln!(
                "If you must kill bough, send SIGKILL and clean up child processes manually."
            );
        })
        .expect("failed to set ctrl-c handler");

        let interactive = std::io::stdout().is_terminal();
        let pb = if interactive {
            let pb = indicatif::ProgressBar::new(total);
            // EMA over per-mutation rate. alpha = smoothing factor; lower = smoother.
            let ema_state = Arc::new(std::sync::Mutex::new(EmaEta::default()));
            pb.set_style(
                indicatif::ProgressStyle::with_template(
                    "{wide_bar:.cyan/blue} {pos}/{len} [{elapsed_precise} elapsed, {ema_eta} remaining]",
                )
                .unwrap()
                .progress_chars("=+ ")
                .with_key(
                    "ema_eta",
                    move |state: &indicatif::ProgressState, w: &mut dyn std::fmt::Write| {
                        let pos = state.pos();
                        let len = state.len().unwrap_or(0);
                        let elapsed = state.elapsed().as_secs_f64();
                        let Ok(mut ema) = ema_state.lock() else {
                            let _ = write!(w, "--:--:--");
                            return;
                        };
                        let rate = ema.update(pos, elapsed);
                        if rate <= 0.0 || len <= pos {
                            let _ = write!(w, "--:--:--");
                            return;
                        }
                        let remaining_secs = ((len - pos) as f64 / rate) as u64;
                        let h = remaining_secs / 3600;
                        let m = (remaining_secs % 3600) / 60;
                        let s = remaining_secs % 60;
                        let _ = write!(w, "{h:02}:{m:02}:{s:02}");
                    },
                ),
            );
            Some(pb)
        } else {
            None
        };

        #[deny(clippy::unwrap_used, clippy::expect_used)]
        let _: () = std::thread::scope(|scope| {
            for workspace_id in workspace_ids {
                let session = Arc::clone(&session);
                let cli = cli.clone();
                let done = done.clone();
                let pb = pb.clone();

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
                    )
                        && !interactive {
                            println!("{}", init.render(&cli));
                        }

                    loop {
                        if done.load(Ordering::Relaxed) {
                            info!(%workspace_id, "shutting down worker");
                            break;
                        }

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
                        )
                            && !interactive {
                                println!("{}", reset.render(&cli));
                            }

                        if let Err(e) =
                            step_apply_mutation::StepApplyMutation::run(&mut workspace, mutation)
                        {
                            error!(%workspace_id, err = %e, "failed to apply mutation");
                            continue;
                        }
                        let test_result = match step_test_mutation::StepTestMutation::run(
                            &workspace,
                            &cli.config,
                            mutation,
                            Some(test_duration),
                        ) {
                            Ok(r) => r,
                            Err(e) => {
                                error!(%workspace_id, err = %e, "test execution failed");
                                let _ =
                                    step_unapply_mutation::StepUnapplyMutation::run(&mut workspace);
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
                        if let Err(e) = guard.set_state(mutation, test_result.status_value.clone())
                        {
                            error!(%workspace_id, err = ?e, "failed to set state");
                        }
                        drop(guard);

                        if let Some(ref pb) = pb {
                            pb.inc(1);
                        }

                        if !interactive {
                            println!("{}", test_result.render(&cli));
                        }
                    }

                    done.store(true, Ordering::Relaxed);
                });
            }
        });

        if let Some(pb) = pb {
            pb.finish();
        }

        Box::new(Noop)
    }
}

mod config;
mod find_best_mutations;
mod render;
mod run;
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

use std::sync::{Arc, Mutex};

use bough_lib::Session;
use bough_typed_hash::UnvalidatedHash;

use config::{Command, Show, parse};
use render::{Noop, Render};
use tracing::{Level, debug, info};

fn main() {
    #[cfg(feature = "pprof")]
    let _pprof_guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

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

                Show::Files { lang: None } => {
                    show_all_files::ShowAllFiles::run(session.lock().unwrap())
                }

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
                } => show_language_mutations::ShowLanguageMutations::run(
                    session.lock().unwrap(),
                    *lang,
                ),

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
                config::Step::TendState => {
                    step_tend_state::StepTendState::run(session.lock().unwrap())
                }

                config::Step::TendWorkspaces => step_tend_workspaces::StepTendWorkspaces::run(
                    session.lock().unwrap(),
                    &cli.config,
                ),

                config::Step::InitWorkspace { workspace_id } => {
                    let guard = session.lock().unwrap();
                    let wid =
                        bough_dirs::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_init_workspace::StepInitWorkspace::run(&workspace, &cli.config, None)
                        .expect("init workspace")
                }

                config::Step::ResetWorkspace { workspace_id } => {
                    let guard = session.lock().unwrap();
                    let wid =
                        bough_dirs::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_reset_workspace::StepResetWorkspace::run(&workspace, &cli.config, None)
                        .expect("reset workspace")
                }

                config::Step::ApplyMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let guard = session.lock().unwrap();
                    let wid =
                        bough_dirs::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let mutation = guard
                        .resolve_mutation(UnvalidatedHash::new(mutation_hash.to_string()))
                        .expect("resolve mutation");
                    let mut workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_apply_mutation::StepApplyMutation::run(&mut workspace, &mutation)
                        .expect("apply mutation")
                }

                config::Step::UnapplyMutation {
                    workspace_id,
                    mutation_hash: _,
                } => {
                    let guard = session.lock().unwrap();
                    let wid =
                        bough_dirs::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let mut workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_unapply_mutation::StepUnapplyMutation::run(&mut workspace)
                        .expect("unapply mutation")
                }

                config::Step::TestMutation {
                    workspace_id,
                    mutation_hash,
                } => {
                    let mut guard = session.lock().unwrap();
                    let wid =
                        bough_dirs::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
                    let mutation = guard
                        .resolve_mutation(UnvalidatedHash::new(mutation_hash.to_string()))
                        .expect("resolve mutation");
                    let mut workspace = guard.bind_workspace(&wid).expect("bind workspace");
                    step_apply_mutation::StepApplyMutation::run(&mut workspace, &mutation)
                        .expect("apply mutation");
                    let result = step_test_mutation::StepTestMutation::run(
                        &workspace,
                        &cli.config,
                        &mutation,
                        None,
                    )
                    .expect("test mutation");
                    step_unapply_mutation::StepUnapplyMutation::run(&mut workspace)
                        .expect("unapply mutation");
                    guard
                        .set_state(&mutation, result.status_value.clone())
                        .expect("set state");
                    result
                }
            }
        }

        Command::Run => run::Run::run(Arc::clone(&session), &cli),

        Command::Find { ref lang, ref file } => find_best_mutations::FindBestMutations::run(
            session.lock().unwrap(),
            *lang,
            file.clone(),
        ),

        Command::Noop => {
            info!("starting run");
            Box::new(Noop)
        }
    };

    println!("{}", result.render(&cli));

    #[cfg(feature = "pprof")]
    {
        use pprof::protos::Message;
        let report = _pprof_guard.report().build().expect("pprof report");
        let profile = report.pprof().expect("pprof profile");
        let mut buf = Vec::new();
        profile.write_to_vec(&mut buf).expect("pprof encode");
        std::fs::write("profile.pb", &buf).expect("write profile.pb");
        eprintln!("wrote profile.pb ({} bytes)", buf.len());
    }
}

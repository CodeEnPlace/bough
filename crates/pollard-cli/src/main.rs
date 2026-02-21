mod action;
mod feedback;
mod session;

use action::Action;
use clap::{Parser, Subcommand};
use feedback::{ApplyRecord, MutationRecord, RenderOutput, Style};
use log::LevelFilter;
use pollard_core::config::{Config, LanguageId, Ordering, Vcs};
use pollard_core::plan::{Plan, PlanEntry};
use session::Session;
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::{
    Hash, Language, MutatedFile, MutationKind, SourceFile, find_mutation_points,
    generate_mutation_substitutions,
};
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, global = true, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long, global = true)]
    language: Option<LanguageId>,

    #[arg(short, long, default_value = "plain", global = true)]
    style: Style,

    #[arg(long, default_value = "unified", global = true)]
    diff: feedback::DiffStyle,

    #[arg(long, env = "NO_COLOR", hide = true, default_value_t = false, global = true)]
    no_color: bool,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[arg(long, global = true)]
    vcs: Option<Vcs>,

    #[arg(long, global = true)]
    working_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    parallelism: Option<usize>,

    #[arg(long, global = true)]
    report_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    ordering: Option<Ordering>,

    #[arg(long, global = true)]
    sub_dir: Option<PathBuf>,

    #[arg(long, global = true)]
    files: Option<String>,

    #[arg(long, global = true)]
    ignore_mutants: Vec<String>,

    #[arg(long, global = true)]
    timeout_absolute: Option<u64>,

    #[arg(long, global = true)]
    timeout_relative: Option<f64>,

    #[arg(long, global = true, default_value_t = false)]
    force_on_dirty_repo: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Mutate {
        #[command(subcommand)]
        action: MutateAction,
    },
    Step {
        #[command(subcommand)]
        action: StepAction,
    },
}

#[derive(Debug, Subcommand)]
enum StepAction {
    Plan,
    Create,
    Apply {
        #[arg(short, long)]
        workspace: String,
        #[arg(long)]
        hash: Hash,
    },
    Install {
        #[arg(short, long)]
        workspace: String,
    },
    Build {
        #[arg(short, long)]
        workspace: String,
    },
    Test {
        #[arg(short, long)]
        workspace: String,
    },
    Reset {
        #[arg(short, long)]
        workspace: String,
        #[arg(short, long)]
        rev: String,
    },
    Cleanup,
}

#[derive(Debug, Subcommand)]
enum MutateAction {
    Generate {
        #[arg(short, long)]
        file: String,
    },
    View {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        hash: Hash,
    },
    Apply {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(long)]
        hash: Hash,
    },
}

fn log_level(cli: &Cli) -> LevelFilter {
    match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

fn generate_for_language<L: Language>(file: &SourceFile) -> Vec<MutationRecord>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    let mut records = Vec::new();
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(point) {
            records.push(MutationRecord {
                source_path: file.path().to_owned(),
                source_hash: file.hash().clone(),
                mutated_hash: mutated.hash().clone(),
                kind: point.kind.into(),
                start_line: point.span.start.line,
                start_char: point.span.start.char,
                end_line: point.span.end.line,
                end_char: point.span.end.char,
                original: file.content()[point.span.start.byte..point.span.end.byte].to_string(),
                replacement,
            });
        }
    }
    records
}

fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    glob::glob(pattern)
        .unwrap_or_else(|e| {
            eprintln!("invalid glob pattern: {e}");
            std::process::exit(1);
        })
        .filter_map(|entry| match entry {
            Ok(path) if path.is_file() => Some(path),
            Ok(_) => None,
            Err(e) => {
                eprintln!("glob error: {e}");
                None
            }
        })
        .collect()
}

fn generate(language: &LanguageId, pattern: &str) -> Vec<MutationRecord> {
    let paths = expand_glob(pattern);
    let mut records = Vec::new();
    for path in &paths {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_records = match language {
            LanguageId::Javascript => generate_for_language::<JavaScript>(&file),
            LanguageId::Typescript => generate_for_language::<TypeScript>(&file),
        };
        records.append(&mut file_records);
    }
    records
}

fn find_mutated_by_hash<'a, L: Language>(
    file: &'a SourceFile,
    target: &Hash,
) -> Option<MutatedFile<'a>> {
    let points = find_mutation_points::<L>(file);
    for point in &points {
        for (_, mutated) in generate_mutation_substitutions::<L>(point) {
            if mutated.hash() == target {
                return Some(mutated);
            }
        }
    }
    None
}

fn view(
    language: &LanguageId,
    input: &Path,
    hash: &Hash,
    diff_style: feedback::DiffStyle,
) -> feedback::DiffRecord {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageId::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageId::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    });

    feedback::DiffRecord {
        old: file.content().to_string(),
        new: mutated.content().to_string(),
        path: file.path().display().to_string(),
        diff_style,
    }
}

fn apply(language: &LanguageId, input: &Path, hash: &Hash) -> (Vec<Action>, ApplyRecord) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = match language {
        LanguageId::Javascript => find_mutated_by_hash::<JavaScript>(&file, hash),
        LanguageId::Typescript => find_mutated_by_hash::<TypeScript>(&file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    });

    let actions = vec![Action::WriteFile {
        path: input.to_owned(),
        content: mutated.content().to_string(),
    }];

    let record = ApplyRecord {
        source_path: file.path().to_owned(),
        source_hash: file.hash().clone(),
        mutated_hash: mutated.hash().clone(),
    };

    (actions, record)
}



type StepResult = (Vec<Action>, Vec<Box<dyn RenderOutput>>);

fn content_id(content: &str) -> String {
    pollard_core::Hash::of(content).to_string()
}

fn plan_entries_for_language<L: Language>(file: &SourceFile) -> Vec<PlanEntry>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    let mut entries = Vec::new();
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(point) {
            entries.push(PlanEntry {
                source_path: file.path().to_owned(),
                source_hash: file.hash().clone(),
                mutated_hash: mutated.hash().clone(),
                kind: point.kind.into(),
                start_line: point.span.start.line,
                start_char: point.span.start.char,
                end_line: point.span.end.line,
                end_char: point.span.end.char,
                start_byte: point.span.start.byte,
                end_byte: point.span.end.byte,
                original: file.content()[point.span.start.byte..point.span.end.byte].to_string(),
                replacement,
            });
        }
    }
    entries
}

fn generate_plan(language: &LanguageId, pattern: &str) -> Plan {
    let paths = expand_glob(pattern);
    let mut entries = Vec::new();
    for path in &paths {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_entries = match language {
            LanguageId::Javascript => plan_entries_for_language::<JavaScript>(&file),
            LanguageId::Typescript => plan_entries_for_language::<TypeScript>(&file),
        };
        entries.append(&mut file_entries);
    }
    Plan { entries }
}

fn step_plan(session: &Session) -> StepResult {
    let plan = generate_plan(&session.language, &session.files);
    let content = serde_json::to_string_pretty(&plan).expect("failed to serialize plan");
    let plan_path = session.working_dir.join(format!("{}.mutants.plan.json", content_id(&content)));

    log::info!("generated {} mutations", plan.entries.len());

    let actions = vec![Action::WriteFile {
        path: plan_path.clone(),
        content,
    }];

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(feedback::PlanRecord {
        path: plan_path,
        count: plan.entries.len(),
    })];

    (actions, renders)
}

fn step_create(session: &Session) -> StepResult {
    match session.vcs {
        Vcs::Jj => {}
        other => {
            eprintln!("step create not yet implemented for {other:?}");
            std::process::exit(1);
        }
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let batch_id = &content_id(&nanos.to_string())[..8];

    let workspaces: Vec<pollard_core::plan::Workspace> = (0..session.parallelism)
        .map(|i| {
            let name = format!("pollard-{batch_id}-{i}");
            let path = session.working_dir.join(&name);
            pollard_core::plan::Workspace { name, path }
        })
        .collect();

    let manifest = pollard_core::plan::WorkspaceManifest {
        workspaces: workspaces.clone(),
    };
    let manifest_content =
        serde_json::to_string_pretty(&manifest).expect("failed to serialize manifest");
    let manifest_path = session
        .working_dir
        .join(format!("{}.workspaces.json", content_id(&manifest_content)));

    let mut actions = vec![Action::WriteFile {
        path: manifest_path.clone(),
        content: manifest_content,
    }];

    for ws in &workspaces {
        actions.push(Action::CreateJjWorkspace {
            name: ws.name.clone(),
            path: ws.path.clone(),
        });
    }

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(feedback::CreateRecord {
        workspaces: workspaces.iter().map(|ws| ws.name.clone()).collect(),
        manifest: manifest_path,
    })];

    (actions, renders)
}

fn read_plan(session: &Session) -> pollard_core::plan::Plan {
    let pattern = session.working_dir.join("*.mutants.plan.json");
    let paths = expand_glob(&pattern.display().to_string());
    let plan_path = paths.first().unwrap_or_else(|| {
        eprintln!("no plan file found in {}", session.working_dir.display());
        std::process::exit(1);
    });
    let content = std::fs::read_to_string(plan_path).unwrap_or_else(|e| {
        eprintln!("failed to read plan: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("failed to parse plan: {e}");
        std::process::exit(1);
    })
}

fn read_workspace_manifest(session: &Session) -> pollard_core::plan::WorkspaceManifest {
    let pattern = session.working_dir.join("*.workspaces.json");
    let paths = expand_glob(&pattern.display().to_string());
    let manifest_path = paths.first().unwrap_or_else(|| {
        eprintln!("no workspace manifest found in {}", session.working_dir.display());
        std::process::exit(1);
    });
    let content = std::fs::read_to_string(manifest_path).unwrap_or_else(|e| {
        eprintln!("failed to read workspace manifest: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("failed to parse workspace manifest: {e}");
        std::process::exit(1);
    })
}

fn step_apply(session: &Session, workspace_name: &str, hash: &Hash) -> StepResult {
    let plan = read_plan(session);
    let manifest = read_workspace_manifest(session);

    let ws = manifest.workspaces.iter().find(|ws| ws.name == workspace_name).unwrap_or_else(|| {
        eprintln!("workspace {workspace_name} not found in manifest");
        std::process::exit(1);
    });

    let entry = plan.entries.iter().find(|e| &e.mutated_hash == hash).unwrap_or_else(|| {
        eprintln!("mutation {hash} not found in plan");
        std::process::exit(1);
    });

    let file_in_workspace = ws.path.join(&entry.source_path);
    let source_content = std::fs::read_to_string(&file_in_workspace).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", file_in_workspace.display());
        std::process::exit(1);
    });

    let source_hash = pollard_core::Hash::of(&source_content);
    if source_hash != entry.source_hash {
        eprintln!(
            "source hash mismatch for {}: expected {}, got {}",
            entry.source_path.display(), entry.source_hash, source_hash,
        );
        std::process::exit(1);
    }

    let mut mutated = String::with_capacity(source_content.len());
    mutated.push_str(&source_content[..entry.start_byte]);
    mutated.push_str(&entry.replacement);
    mutated.push_str(&source_content[entry.end_byte..]);

    std::fs::write(&file_in_workspace, &mutated).unwrap_or_else(|e| {
        eprintln!("failed to write {}: {e}", file_in_workspace.display());
        std::process::exit(1);
    });

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(ApplyRecord {
        source_path: file_in_workspace,
        source_hash: entry.source_hash.clone(),
        mutated_hash: entry.mutated_hash.clone(),
    })];

    (vec![], renders)
}

fn run_in_workspace(
    session: &Session,
    workspace_name: &str,
    command: &Option<String>,
    step_name: &str,
) -> StepResult {
    let cmd = match command {
        Some(c) => c,
        None => {
            log::info!("no {step_name} command configured, skipping");
            return (vec![], vec![]);
        }
    };

    let manifest = read_workspace_manifest(session);
    let ws = manifest.workspaces.iter().find(|ws| ws.name == workspace_name).unwrap_or_else(|| {
        eprintln!("workspace {workspace_name} not found in manifest");
        std::process::exit(1);
    });

    log::info!("running {step_name} in {}: {cmd}", ws.path.display());

    let output = std::process::Command::new("sh")
        .args(["-c", cmd])
        .current_dir(ws.path.join(&session.sub_dir))
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run {step_name}: {e}");
            std::process::exit(1);
        });

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let result = if output.status.success() {
        Ok(stdout)
    } else {
        Err(stderr)
    };

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(feedback::CommandRecord {
        step: step_name.to_string(),
        workspace: workspace_name.to_string(),
        command: cmd.to_string(),
        result,
    })];

    (vec![], renders)
}

fn step_install(session: &Session, workspace: &str) -> StepResult {
    run_in_workspace(session, workspace, &session.commands.install, "install")
}

fn step_build(session: &Session, workspace: &str) -> StepResult {
    run_in_workspace(session, workspace, &session.commands.build, "build")
}

fn step_test(session: &Session, workspace: &str) -> StepResult {
    run_in_workspace(session, workspace, &Some(session.commands.test.clone()), "test")
}

fn step_reset(session: &Session, workspace_name: &str, rev: &str) -> StepResult {
    match session.vcs {
        Vcs::Jj => {}
        other => {
            eprintln!("step reset not yet implemented for {other:?}");
            std::process::exit(1);
        }
    }

    let manifest = read_workspace_manifest(session);
    let ws = manifest.workspaces.iter().find(|ws| ws.name == workspace_name).unwrap_or_else(|| {
        eprintln!("workspace {workspace_name} not found in manifest");
        std::process::exit(1);
    });

    log::info!("resetting workspace {} to {rev}", ws.name);
    let output = std::process::Command::new("jj")
        .args(["edit", rev])
        .current_dir(&ws.path)
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run jj edit: {e}");
            std::process::exit(1);
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("jj edit failed for workspace {}: {stderr}", ws.name);
        std::process::exit(1);
    }

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(feedback::ResetRecord {
        workspace: workspace_name.to_string(),
        rev: rev.to_string(),
    })];

    (vec![], renders)
}

fn step_cleanup(session: &Session) -> StepResult {
    let pattern = session.working_dir.join("*.workspaces.json");
    let manifest_paths = expand_glob(&pattern.display().to_string());

    let mut actions: Vec<Action> = Vec::new();
    let mut workspace_count = 0;

    for manifest_path in &manifest_paths {
        let content = std::fs::read_to_string(manifest_path)
            .unwrap_or_else(|e| {
                eprintln!("failed to read {}: {e}", manifest_path.display());
                std::process::exit(1);
            });
        let manifest: pollard_core::plan::WorkspaceManifest = serde_json::from_str(&content)
            .unwrap_or_else(|e| {
                eprintln!("failed to parse {}: {e}", manifest_path.display());
                std::process::exit(1);
            });

        for ws in &manifest.workspaces {
            actions.push(Action::ForgetJjWorkspace { name: ws.name.clone() });
            actions.push(Action::RemoveDir { path: ws.path.clone() });
            workspace_count += 1;
        }

        actions.push(Action::RemoveFile { path: manifest_path.clone() });
    }

    let renders: Vec<Box<dyn RenderOutput>> = vec![Box::new(feedback::CleanupRecord {
        workspace_count,
        manifest_count: manifest_paths.len(),
    })];

    (actions, renders)
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .parse_default_env()
        .filter_level(log_level(&cli))
        .init();

    let cwd = std::env::current_dir().expect("failed to get current directory");
    let discovered = match &cli.config {
        Some(path) => Some((path.clone(), Config::read(path))),
        None => Config::discover(&cwd),
    };
    let (config_path, config) = match discovered {
        Some((path, Ok(config))) => {
            log::info!("loaded config from {}", path.display());
            (path, config)
        }
        Some((path, Err(e))) => {
            eprintln!("error in {}: {e}", path.display());
            std::process::exit(1);
        }
        None => {
            eprintln!("no config file found (searched from {})", cwd.display());
            std::process::exit(1);
        }
    };
    log::debug!("cli: {cli:?}");
    log::debug!("config: {}", serde_json::to_string(&config).expect("failed to serialize config"));

    let session = Session::from_cli_and_config(&cli, config, config_path).unwrap_or_else(|e| {
        eprintln!("{e}");
        std::process::exit(1);
    });
    log::info!("session: {}", serde_json::to_string(&session).expect("failed to serialize session"));

    match &cli.command {
        Command::Mutate { action } => {
            let (actions, renders): (Vec<Action>, Vec<Box<dyn RenderOutput>>) = match action {
                MutateAction::Generate { file: pattern } => {
                    let renders: Vec<Box<dyn RenderOutput>> = generate(&session.language, pattern)
                        .into_iter()
                        .map(|r| Box::new(r) as Box<dyn RenderOutput>)
                        .collect();
                    (vec![], renders)
                }
                MutateAction::View { file: input, hash } => {
                    let record = view(&session.language, input, hash, session.diff.clone());
                    (vec![], vec![Box::new(record)])
                }
                MutateAction::Apply { file: input, hash } => {
                    let (actions, record) = apply(&session.language, input, hash);
                    (actions, vec![Box::new(record)])
                }
            };

            if !actions.is_empty() && !session.force_on_dirty_repo && action::repo_is_dirty() {
                eprintln!("repo has uncommitted changes, use --force-on-dirty-repo to proceed");
                std::process::exit(1);
            }

            for action in actions {
                action.apply().expect("failed to apply action");
            }

            for render in &renders {
                render.render(&session.style, session.no_color);
            }
        }
        Command::Step { action } => {
            let (actions, renders): (Vec<Action>, Vec<Box<dyn RenderOutput>>) = match action {
                StepAction::Plan => step_plan(&session),
                StepAction::Create => step_create(&session),
                StepAction::Apply { workspace, hash } => step_apply(&session, workspace, hash),
                StepAction::Install { workspace } => step_install(&session, workspace),
                StepAction::Build { workspace } => step_build(&session, workspace),
                StepAction::Test { workspace } => step_test(&session, workspace),
                StepAction::Reset { workspace, rev } => step_reset(&session, workspace, rev),
                StepAction::Cleanup => step_cleanup(&session),
            };

            if !actions.is_empty() && !session.force_on_dirty_repo && action::repo_is_dirty() {
                eprintln!("repo has uncommitted changes, use --force-on-dirty-repo to proceed");
                std::process::exit(1);
            }

            for action in actions {
                action.apply().expect("failed to apply action");
            }

            for render in &renders {
                render.render(&session.style, session.no_color);
            }
        }
    }
}

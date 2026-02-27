use bough_core::config::Config;
use bough_core::{Mutation, MutationResult, Outcome};
use bough_typed_hash::{DiskHashStore, HashStore, MemoryHashStore, TypedHashable};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::phase_runner::{self, PhaseRunner};
use crate::render::{Render, color};

use super::{get_mutations, get_src_files};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    NoTestPhase,
    WorkspaceNotFound(PathBuf),
    SrcFiles(get_src_files::Error),
    Mutations(get_mutations::Error),
    MutationNotFound(String),
    Phase(phase_runner::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::NoTestPhase => write!(f, "no test phase configured for runner"),
            Error::WorkspaceNotFound(p) => write!(f, "workspace not found: {}", p.display()),
            Error::SrcFiles(e) => write!(f, "{e}"),
            Error::Mutations(e) => write!(f, "{e}"),
            Error::MutationNotFound(h) => write!(f, "no mutation found with hash {h}"),
            Error::Phase(e) => write!(f, "test phase failed: {e}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct TestWorkspace {
    pub workspace: PathBuf,
    pub mutation_hash: String,
    pub outcome: Outcome,
}

fn find_mutation(config: &Config, hash: &str) -> Result<Mutation, Error> {
    let src_files = get_src_files::run(config).map_err(Error::SrcFiles)?;
    let mutations = get_mutations::run(&src_files, config).map_err(Error::Mutations)?;

    for (_lang, muts) in &mutations.mutations {
        for m in muts {
            let m_hash = m.hash(&mut MemoryHashStore::new()).expect("hash failed").to_string();
            if m_hash == hash {
                return Ok(m.clone());
            }
        }
    }

    Err(Error::MutationNotFound(hash.to_string()))
}

pub fn run(config: &Config, workspace: &Path, mutation_hash: &str) -> Result<TestWorkspace, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let runner = config.runner(runner_name);
    let test_phase = config.runner_test_phase(runner_name).ok_or(Error::NoTestPhase)?;

    if !workspace.exists() {
        return Err(Error::WorkspaceNotFound(workspace.to_path_buf()));
    }

    let mutation = find_mutation(config, mutation_hash)?;

    let outcome = match PhaseRunner::new(config, runner, test_phase, workspace).run() {
        Ok(_) => Outcome::Missed,
        Err(phase_runner::Error::CommandFailed { .. }) => Outcome::Caught,
        Err(phase_runner::Error::Timeout { .. }) => {
            config.runner_treat_timeouts_as(runner_name).unwrap_or_default()
        }
        Err(e) => return Err(Error::Phase(e)),
    };

    let result = MutationResult {
        outcome,
        mutation,
        at: chrono::Utc::now(),
    };

    let mut store = DiskHashStore::<MutationResult>::new(
        PathBuf::from(config.state_dir()),
    );
    store.insert(result);

    Ok(TestWorkspace {
        workspace: workspace.to_path_buf(),
        mutation_hash: mutation_hash.to_string(),
        outcome,
    })
}

impl Render for TestWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "{} mutation {} in workspace {}\n",
            match self.outcome {
                Outcome::Caught => color("\x1b[32m", "caught"),
                Outcome::Missed => color("\x1b[31m", "missed"),
            },
            color("\x1b[2m", &self.mutation_hash),
            color("\x1b[1m", &self.workspace.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        self.render_terse()
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        format!(
            "{heading} Test Workspace\n\n\
             - **Workspace:** `{}`\n\
             - **Mutation:** `{}`\n\
             - **Outcome:** {:?}\n",
            self.workspace.display(),
            self.mutation_hash,
            self.outcome,
        )
    }
}

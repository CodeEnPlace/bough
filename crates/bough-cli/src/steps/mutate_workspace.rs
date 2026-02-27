use bough_core::apply_mutation;
use bough_core::config::Config;
use bough_core::Mutation;
use bough_typed_hash::{MemoryHashStore, TypedHashable};
use serde::Serialize;
use std::path::{Path, PathBuf};

use super::get_src_files;
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    WorkspaceNotFound(PathBuf),
    MutationNotFound(String),
    SrcFiles(get_src_files::Error),
    Mutations(super::get_mutations::Error),
    ReadFile(PathBuf, std::io::Error),
    WriteFile(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::WorkspaceNotFound(p) => write!(f, "workspace not found: {}", p.display()),
            Error::MutationNotFound(h) => write!(f, "no mutation found with hash {h}"),
            Error::SrcFiles(e) => write!(f, "{e}"),
            Error::Mutations(e) => write!(f, "{e}"),
            Error::ReadFile(p, e) => write!(f, "failed to read {}: {e}", p.display()),
            Error::WriteFile(p, e) => write!(f, "failed to write {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct MutateWorkspace {
    pub workspace: PathBuf,
    pub file: PathBuf,
    pub hash: String,
    pub replacement: String,
}

fn find_mutation(config: &Config, hash: &str) -> Result<Mutation, Error> {
    let src_files = get_src_files::run(config).map_err(Error::SrcFiles)?;
    let mutations = super::get_mutations::run(&src_files, config).map_err(Error::Mutations)?;

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

fn workspace_file_path(workspace: &Path, mutation_path: &Path, config: &Config) -> PathBuf {
    let cwd = std::env::current_dir().unwrap();
    let relative = mutation_path
        .strip_prefix(&cwd)
        .unwrap_or(mutation_path);
    dbg!(&workspace, &mutation_path, &cwd, &relative);
    workspace.join(relative)
}

pub fn run(config: &Config, workspace: &Path, hash: &str) -> Result<MutateWorkspace, Error> {
    config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;

    if !workspace.exists() {
        return Err(Error::WorkspaceNotFound(workspace.to_path_buf()));
    }

    let mutation = find_mutation(config, hash)?;
    let file_path = workspace_file_path(workspace, &mutation.mutant.src.path, config);

    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| Error::ReadFile(file_path.clone(), e))?;

    let mutated = apply_mutation(&content, &mutation.mutant.span, &mutation.replacement);

    std::fs::write(&file_path, &mutated)
        .map_err(|e| Error::WriteFile(file_path.clone(), e))?;

    Ok(MutateWorkspace {
        workspace: workspace.to_path_buf(),
        file: file_path,
        hash: hash.to_string(),
        replacement: mutation.replacement,
    })
}

impl Render for MutateWorkspace {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "applied mutation {} to {}\n",
            color("\x1b[2m", &self.hash),
            color("\x1b[1m", &self.file.display().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        format!(
            "applied mutation {} to {}\nreplacement: {}\n",
            color("\x1b[2m", &self.hash),
            color("\x1b[1m", &self.file.display().to_string()),
            color("\x1b[32m", &self.replacement),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        format!(
            "{heading} Mutate Workspace\n\n\
             - **Workspace:** `{}`\n\
             - **File:** `{}`\n\
             - **Hash:** `{}`\n\
             - **Replacement:** `{}`\n",
            self.workspace.display(),
            self.file.display(),
            self.hash,
            self.replacement,
        )
    }
}

use bough_core::{MutationHash, MutationResult};
use bough_core::config::Config;
use bough_typed_hash::{DiskHashStore, HashStore, TypedHash};
use serde::Serialize;
use std::path::PathBuf;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NotFound(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound(h) => write!(f, "no mutation result found for hash {h}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Serialize)]
pub struct ShowMutationResult {
    pub result: MutationResult,
    pub hash: String,
}

pub fn run(config: &Config, hash: &str) -> Result<ShowMutationResult, Error> {
    let store = DiskHashStore::<MutationResult>::new(PathBuf::from(config.state_dir()));

    let typed_hash = MutationHash::parse::<MutationResult>(hash, &store)
        .map_err(|_| Error::NotFound(hash.to_string()))?;

    let result = store
        .get(&typed_hash)
        .ok_or_else(|| Error::NotFound(hash.to_string()))?;

    Ok(ShowMutationResult {
        result: result.clone(),
        hash: hash.to_string(),
    })
}

impl Render for ShowMutationResult {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(&self.result).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        let outcome = match self.result.outcome {
            bough_core::Outcome::Caught => color("\x1b[32m", "caught"),
            bough_core::Outcome::Missed => color("\x1b[31m", "missed"),
        };
        format!(
            "mutation {} {}\n",
            color("\x1b[2m", &self.hash),
            outcome,
        )
    }

    fn render_verbose(&self) -> String {
        let outcome = match self.result.outcome {
            bough_core::Outcome::Caught => color("\x1b[32m", "caught"),
            bough_core::Outcome::Missed => color("\x1b[31m", "missed"),
        };
        let m = &self.result.mutation;
        format!(
            "mutation {} {}\n  file: {}:{}\n  kind: {:?}\n  replacement: {}\n  tested: {}\n",
            color("\x1b[2m", &self.hash),
            outcome,
            m.mutant.src.path.display(),
            m.mutant.span.start.line + 1,
            m.mutant.kind,
            color("\x1b[32m", &m.replacement),
            self.result.at,
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let m = &self.result.mutation;
        format!(
            "{heading} Mutation Result\n\n\
             - **Hash:** `{}`\n\
             - **Outcome:** {:?}\n\
             - **File:** `{}:{}`\n\
             - **Kind:** {:?}\n\
             - **Replacement:** `{}`\n\
             - **Tested:** {}\n",
            self.hash,
            self.result.outcome,
            m.mutant.src.path.display(),
            m.mutant.span.start.line + 1,
            m.mutant.kind,
            m.replacement,
            self.result.at,
        )
    }
}

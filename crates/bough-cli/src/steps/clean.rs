use bough_core::config::Config;
use bough_typed_hash::{MemoryHashStore, TypedHashable};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;

use super::{get_mutations, get_src_files};
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    SrcFiles(get_src_files::Error),
    Mutations(get_mutations::Error),
    ReadDir(std::io::Error),
    Remove(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::SrcFiles(e) => write!(f, "{e}"),
            Error::Mutations(e) => write!(f, "{e}"),
            Error::ReadDir(e) => write!(f, "failed to read state dir: {e}"),
            Error::Remove(p, e) => write!(f, "failed to remove {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct Clean {
    pub removed: Vec<String>,
    pub kept: usize,
}

pub fn run(config: &Config) -> Result<Clean, Error> {
    let src_files = get_src_files::run(config).map_err(Error::SrcFiles)?;
    let mutations = get_mutations::run(&src_files, config).map_err(Error::Mutations)?;

    let mut valid_hashes = HashSet::new();
    let mut store = MemoryHashStore::new();
    for (_lang, muts) in &mutations.mutations {
        for m in muts {
            let hash = m.hash(&mut store).expect("hash failed");
            valid_hashes.insert(hash.to_string());
        }
    }

    let state_dir = PathBuf::from(config.state_dir());
    let mut removed = Vec::new();
    let mut kept = 0;

    let entries = match std::fs::read_dir(&state_dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Clean { removed, kept });
        }
        Err(e) => return Err(Error::ReadDir(e)),
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(hex) = name.strip_suffix(".json") else {
            continue;
        };

        if valid_hashes.contains(hex) {
            kept += 1;
        } else {
            let path = entry.path();
            std::fs::remove_file(&path).map_err(|e| Error::Remove(path.clone(), e))?;
            removed.push(hex.to_string());
        }
    }

    Ok(Clean { removed, kept })
}

impl Render for Clean {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(self).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "removed {} stale results, kept {}\n",
            color("\x1b[1m", &self.removed.len().to_string()),
            color("\x1b[1m", &self.kept.to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = self.render_terse();
        for hash in &self.removed {
            out.push_str(&format!("  removed {}\n", color("\x1b[2m", hash)));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let mut out = format!(
            "{heading} Clean\n\n- **Removed:** {}\n- **Kept:** {}\n",
            self.removed.len(),
            self.kept,
        );
        if !self.removed.is_empty() {
            out.push_str("\nRemoved hashes:\n");
            for hash in &self.removed {
                out.push_str(&format!("- `{hash}`\n"));
            }
        }
        out.push('\n');
        out
    }
}

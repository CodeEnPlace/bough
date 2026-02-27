use bough_core::config::{Config, MutantSkip};
use bough_core::languages::LanguageId;
use bough_core::{
    Mutation, MutationKind, SourceFile, filter_mutants, find_mutants, generate_mutations,
};
use bough_typed_hash::{TypedHashable, MemoryHashStore};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

use super::get_src_files::ShowSrcFiles;
use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    ReadFile(PathBuf, std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ReadFile(path, err) => write!(f, "failed to read {}: {err}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct ShowMutations {
    pub mutations: BTreeMap<LanguageId, Vec<Mutation>>,
}

fn collect_mutations(
    files: &[SourceFile],
    skip: &[MutantSkip],
) -> Result<Vec<Mutation>, Error> {
    let queries: Vec<String> = skip
        .iter()
        .filter_map(|s| match s {
            MutantSkip::Query { query } => Some(query.clone()),
            _ => None,
        })
        .collect();

    let mut results = Vec::new();
    for file in files {
        let content = std::fs::read_to_string(&file.path)
            .map_err(|e| Error::ReadFile(file.path.clone(), e))?;
        let mutants = find_mutants(file, &content);
        let mutants = filter_mutants(mutants, &queries, &content);
        for mutant in &mutants {
            for mutation in generate_mutations(mutant) {
                results.push(mutation);
            }
        }
    }
    Ok(results)
}

pub fn run(src_files: &ShowSrcFiles, config: &Config) -> Result<ShowMutations, Error> {
    let runner_name = config.resolved_runner_name();

    let mut mutations = BTreeMap::new();

    for (lang, files) in &src_files.files {
        let skips = runner_name
            .map(|r| config.mutant_skips(r, *lang))
            .unwrap_or_default();

        let lang_mutations = collect_mutations(files, &skips)?;
        mutations.insert(*lang, lang_mutations);
    }

    Ok(ShowMutations { mutations })
}

fn mutation_hash_hex(m: &Mutation) -> String {
    m.hash(&mut MemoryHashStore::new()).expect("hash failed").to_string()
}

impl Render for ShowMutations {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(&self.mutations).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        let mut out = String::new();
        for (lang, mutations) in &self.mutations {
            out.push_str(&format!(
                "found {} mutations for {}\n",
                color("\x1b[1m", &mutations.len().to_string()),
                color("\x1b[36m", &format!("{lang:?}")),
            ));
        }
        out
    }

    fn render_verbose(&self) -> String {
        let mut out = String::new();
        for (lang, mutations) in &self.mutations {
            out.push_str(&color(
                "\x1b[1m",
                &format!("{lang:?} ({} mutations)", mutations.len()),
            ));
            out.push('\n');
            for m in mutations {
                let kind: MutationKind = m.mutant.kind.clone();
                out.push_str(&format!(
                    "  {} {}:{} {} → {}\n",
                    color("\x1b[2m", &mutation_hash_hex(m)),
                    m.mutant.src.path.display(),
                    m.mutant.span.start.line + 1,
                    color("\x1b[33m", &format!("{kind:?}")),
                    color("\x1b[32m", &m.replacement),
                ));
            }
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let h1 = "#".repeat((depth + 1).min(6) as usize);
        let h2 = "#".repeat((depth + 2).min(6) as usize);
        let mut out = format!("{h1} Mutations\n\n");
        for (lang, mutations) in &self.mutations {
            out.push_str(&format!(
                "{h2} {lang:?} ({} mutations)\n\n",
                mutations.len()
            ));
            out.push_str("| File | Line | Kind | Replacement |\n");
            out.push_str("|------|------|------|-------------|\n");
            for m in mutations {
                let kind: MutationKind = m.mutant.kind.clone();
                out.push_str(&format!(
                    "| `{}` | {} | {:?} | `{}` |\n",
                    m.mutant.src.path.display(),
                    m.mutant.span.start.line + 1,
                    kind,
                    m.replacement,
                ));
            }
            out.push('\n');
        }
        out
    }
}

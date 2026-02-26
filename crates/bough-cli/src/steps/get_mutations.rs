use bough_core::config::{Config, MutantSkip};
use bough_core::languages::LanguageId;
use bough_core::{
    Language, Mutation, MutationKind, SourceFile, filter_mutants, find_mutants,
    generate_mutations,
    languages::{JavaScript, TypeScript},
};
use bough_sha::ShaHashable;
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

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum AnyMutation {
    Js(Mutation<JavaScript>),
    Ts(Mutation<TypeScript>),
}

impl AnyMutation {
    pub fn path(&self) -> &std::path::Path {
        match self {
            AnyMutation::Js(m) => &m.mutant.src.path,
            AnyMutation::Ts(m) => &m.mutant.src.path,
        }
    }

    pub fn line(&self) -> usize {
        match self {
            AnyMutation::Js(m) => m.mutant.span.start.line + 1,
            AnyMutation::Ts(m) => m.mutant.span.start.line + 1,
        }
    }

    pub fn kind(&self) -> MutationKind {
        match self {
            AnyMutation::Js(m) => m.mutant.kind.clone().into(),
            AnyMutation::Ts(m) => m.mutant.kind.clone().into(),
        }
    }

    pub fn replacement(&self) -> &str {
        match self {
            AnyMutation::Js(m) => &m.replacement,
            AnyMutation::Ts(m) => &m.replacement,
        }
    }

    pub fn hash_hex(&self) -> String {
        match self {
            AnyMutation::Js(m) => m.sha_hash().to_string(),
            AnyMutation::Ts(m) => m.sha_hash().to_string(),
        }
    }

    pub fn span(&self) -> &bough_core::Span {
        match self {
            AnyMutation::Js(m) => &m.mutant.span,
            AnyMutation::Ts(m) => &m.mutant.span,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ShowMutations {
    pub mutations: BTreeMap<LanguageId, Vec<AnyMutation>>,
}

fn collect_mutations<L: Language>(
    files: &[SourceFile],
    skip: &[MutantSkip],
    wrap: fn(Mutation<L>) -> AnyMutation,
) -> Result<Vec<AnyMutation>, Error>
where
    L::Kind: Clone + Into<MutationKind>,
{
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
        let mutants = find_mutants::<L>(file, &content);
        let mutants = filter_mutants::<L>(mutants, &queries, &content);
        for mutant in &mutants {
            for mutation in generate_mutations::<L>(mutant) {
                results.push(wrap(mutation));
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

        let lang_mutations = match lang {
            LanguageId::Javascript => collect_mutations::<JavaScript>(files, &skips, AnyMutation::Js)?,
            LanguageId::Typescript => collect_mutations::<TypeScript>(files, &skips, AnyMutation::Ts)?,
        };
        mutations.insert(*lang, lang_mutations);
    }

    Ok(ShowMutations { mutations })
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
                out.push_str(&format!(
                    "  {} {}:{} {} → {}\n",
                    color("\x1b[2m", &m.hash_hex()),
                    m.path().display(),
                    m.line(),
                    color("\x1b[33m", &format!("{:?}", m.kind())),
                    color("\x1b[32m", m.replacement()),
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
                out.push_str(&format!(
                    "| `{}` | {} | {:?} | `{}` |\n",
                    m.path().display(),
                    m.line(),
                    m.kind(),
                    m.replacement(),
                ));
            }
            out.push('\n');
        }
        out
    }
}

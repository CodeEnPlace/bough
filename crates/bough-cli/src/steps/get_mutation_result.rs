use bough_core::languages::{LanguageId, driver_for};
use bough_core::{MutationHash, MutationResult, apply_mutation};
use bough_core::config::Config;
use bough_typed_hash::{DiskHashStore, HashStore, TypedHash};
use serde::Serialize;
use std::path::PathBuf;
use tree_sitter::Parser;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NotFound(String),
    SourceFileNotFound(PathBuf),
    SourceFileChanged(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound(h) => write!(f, "no mutation result found for hash {h}"),
            Error::SourceFileNotFound(p) => write!(f, "source file not found: {}", p.display()),
            Error::SourceFileChanged(p) => write!(f, "source file has changed: {}", p.display()),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Serialize)]
pub struct ShowMutationResult {
    pub result: MutationResult,
    pub hash: String,
    #[serde(skip)]
    pub context: MutationContext,
}

#[derive(Default)]
pub struct MutationContext {
    pub before: String,
    pub diff: String,
    pub lang_fence: &'static str,
}

fn build_diff(before: &[&str], after: &[&str]) -> String {
    let mut out = String::new();
    let max = before.len().max(after.len());
    let mut i = 0;
    let mut j = 0;
    while i < before.len() || j < after.len() {
        if i < before.len() && j < after.len() && before[i] == after[j] {
            out.push_str(&format!(" {}\n", before[i]));
            i += 1;
            j += 1;
        } else {
            while i < before.len() && (j >= after.len() || (i < max && before.get(i) != after.get(j))) {
                out.push_str(&format!("-{}\n", before[i]));
                i += 1;
            }
            while j < after.len() && (i >= before.len() || (j < max && before.get(i) != after.get(j))) {
                out.push_str(&format!("+{}\n", after[j]));
                j += 1;
            }
        }
    }
    out.trim_end_matches('\n').to_string()
}

fn lang_fence(lang: LanguageId) -> &'static str {
    match lang {
        LanguageId::Javascript => "javascript",
        LanguageId::Typescript => "typescript",
    }
}

fn extract_lines(content: &str, start: usize, end: usize) -> String {
    content
        .lines()
        .skip(start)
        .take(end - start + 1)
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_context(result: &MutationResult) -> Result<MutationContext, Error> {
    let mutant = &result.mutation.mutant;
    let path = &mutant.src.path;

    let content = std::fs::read_to_string(path)
        .map_err(|_| Error::SourceFileNotFound(path.clone()))?;

    let driver = driver_for(mutant.src.language);
    let mut parser = Parser::new();
    parser
        .set_language(&driver.tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser.parse(&content, None).expect("failed to parse source");

    let node = mutant
        .to_ts_node(&tree)
        .ok_or_else(|| Error::SourceFileChanged(path.clone()))?;

    let mutant_start = mutant.span.start.line;
    let mutant_end = mutant.span.end.line;
    let line_count = content.lines().count();

    let mut ctx_start = node.start_position().row;
    let mut ctx_end = node.end_position().row;

    let mut current = node;
    while let Some(parent) = current.parent() {
        ctx_start = ctx_start.min(parent.start_position().row);
        ctx_end = ctx_end.max(parent.end_position().row);

        let above = mutant_start.saturating_sub(ctx_start);
        let below = ctx_end.saturating_sub(mutant_end);
        if above >= 2 && below >= 2 {
            break;
        }
        current = parent;
    }

    ctx_end = ctx_end.min(line_count.saturating_sub(1));

    let before = extract_lines(&content, ctx_start, ctx_end);

    let mutated = apply_mutation(&content, &mutant.span, &result.mutation.replacement);
    let replacement_line_delta =
        result.mutation.replacement.lines().count() as isize
            - extract_lines(&content, mutant_start, mutant_end)
                .lines()
                .count() as isize;
    let after_end = (ctx_end as isize + replacement_line_delta).max(ctx_start as isize) as usize;
    let after = extract_lines(&mutated, ctx_start, after_end);

    let before_lines: Vec<&str> = before.lines().collect();
    let after_lines: Vec<&str> = after.lines().collect();
    let diff = build_diff(&before_lines, &after_lines);

    Ok(MutationContext {
        before,
        diff,
        lang_fence: lang_fence(mutant.src.language),
    })
}

pub fn run(config: &Config, hash: &str) -> Result<ShowMutationResult, Error> {
    let store = DiskHashStore::<MutationResult>::new(PathBuf::from(config.state_dir()));

    let typed_hash = MutationHash::parse::<MutationResult>(hash, &store)
        .map_err(|_| Error::NotFound(hash.to_string()))?;

    let result = store
        .get(&typed_hash)
        .ok_or_else(|| Error::NotFound(hash.to_string()))?
        .clone();

    let context = build_context(&result)?;

    Ok(ShowMutationResult {
        result,
        hash: hash.to_string(),
        context,
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
        let ctx = &self.context;
        format!(
            "{heading} Mutation Result\n\n\
             - **Hash:** `{}`\n\
             - **Outcome:** {:?}\n\
             - **File:** `{}:{}`\n\
             - **Kind:** {:?}\n\
             - **Replacement:** `{}`\n\
             - **Tested:** {}\n\n\
             {heading}# Before\n\n\
             ```{}\n\
             {}\n\
             ```\n\n\
             {heading}# Diff\n\n\
             ```diff\n\
             {}\n\
             ```\n",
            self.hash,
            self.result.outcome,
            m.mutant.src.path.display(),
            m.mutant.span.start.line + 1,
            m.mutant.kind,
            m.replacement,
            self.result.at,
            ctx.lang_fence,
            ctx.before,
            ctx.diff,
        )
    }
}

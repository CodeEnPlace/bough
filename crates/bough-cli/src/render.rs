use std::path::PathBuf;

use bough_core::{LanguageId, Mutation, State};
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};
use facet::Facet;

use crate::config::{Cli, Config, Format};

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

const RESET: &str = "\x1b[0m";
const TITLE: &str = "\x1b[33m";
const PATH: &str = "\x1b[34m";
const HASH: &str = "\x1b[36m";
const LANG: &str = "\x1b[35m";
const STRING: &str = "\x1b[33m";

pub trait Render {
    fn markdown(&self) -> String;
    fn terse(&self) -> String;
    fn verbose(&self) -> String;
    fn json(&self) -> String;

    fn render(&self, cli: &Cli) -> String {
        let out = match cli.format {
            Format::Terse => self.terse(),
            Format::Verbose => self.verbose(),
            Format::Markdown => self.markdown(),
            Format::Json => self.json(),
        };
        if cli.color() { out } else { strip_ansi(&out) }
    }
}

#[derive(Facet)]
pub struct Noop;
impl Render for Noop {
    fn markdown(&self) -> String {
        String::new()
    }

    fn terse(&self) -> String {
        todo!()
    }

    fn verbose(&self) -> String {
        todo!()
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct BaseFiles(pub Vec<PathBuf>);
impl Render for BaseFiles {
    fn markdown(&self) -> String {
        format!(
            "# Files in Base Directory\n\n\tThese files will be coppied into Workspace directories\n\n{}",
            self.0
                .iter()
                .map(|pb| format!("- {}", pb.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct MutantFiles(pub LanguageId, pub Vec<PathBuf>);
impl Render for MutantFiles {
    fn markdown(&self) -> String {
        format!(
            "# {:?} Files that will be Mutated\n\n{}",
            self.0,
            self.1
                .iter()
                .map(|pb| format!("- {}", pb.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    fn terse(&self) -> String {
        self.1
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.1
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

fn mutation_hash(m: &Mutation) -> String {
    let hash = m.hash().expect("hashing should not fail");
    format!("{hash}")
}

fn fmt_mutation_terse(m: &Mutation) -> String {
    let mutant = m.mutant();
    format!(
        "{HASH}{}{RESET} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
        mutation_hash(m),
        mutant.twig().path().display(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.kind(),
        m.subst(),
    )
}

fn fmt_mutation_markdown_row(m: &Mutation) -> String {
    let mutant = m.mutant();
    format!(
        "| `{}` | {} | {:?} | {:?} | {}:{}-{}:{} | `{}` |",
        mutation_hash(m),
        mutant.twig().path().display(),
        mutant.lang(),
        mutant.kind(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.span().end().line() + 1,
        mutant.span().end().col() + 1,
        m.subst(),
    )
}

const MARKDOWN_TABLE_HEADER: &str = "| Hash | File | Lang | Kind | Span | Subst |\n| --- | --- | --- | --- | --- | --- |";

fn fmt_mutation_markdown_table(mutations: &[Mutation]) -> String {
    let rows: Vec<_> = mutations.iter().map(fmt_mutation_markdown_row).collect();
    format!("{MARKDOWN_TABLE_HEADER}\n{}", rows.join("\n"))
}

fn fmt_mutation_verbose(m: &Mutation) -> String {
    let mutant = m.mutant();
    format!(
        "{HASH}{}{RESET} {PATH}{}{RESET} [{LANG}{:?}{RESET}] {:?} @ {}:{}-{}:{} → {STRING}\"{}\"{RESET}",
        mutation_hash(m),
        mutant.twig().path().display(),
        mutant.lang(),
        mutant.kind(),
        mutant.span().start().line() + 1,
        mutant.span().start().col() + 1,
        mutant.span().end().line() + 1,
        mutant.span().end().col() + 1,
        m.subst(),
    )
}

#[derive(Facet)]
pub struct AllMutations(pub Vec<Mutation>);
impl Render for AllMutations {
    fn markdown(&self) -> String {
        format!(
            "# All Mutations\n\n{} total\n\n{}",
            self.0.len(),
            fmt_mutation_markdown_table(&self.0),
        )
    }

    fn terse(&self) -> String {
        self.0.iter().map(fmt_mutation_terse).collect::<Vec<_>>().join("\n")
    }

    fn verbose(&self) -> String {
        self.0.iter().map(fmt_mutation_verbose).collect::<Vec<_>>().join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct LangMutations(pub LanguageId, pub Vec<Mutation>);
impl Render for LangMutations {
    fn markdown(&self) -> String {
        format!(
            "# {:?} Mutations\n\n{} total\n\n{}",
            self.0,
            self.1.len(),
            fmt_mutation_markdown_table(&self.1),
        )
    }

    fn terse(&self) -> String {
        self.1.iter().map(fmt_mutation_terse).collect::<Vec<_>>().join("\n")
    }

    fn verbose(&self) -> String {
        self.1.iter().map(fmt_mutation_verbose).collect::<Vec<_>>().join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[derive(Facet)]
pub struct FileMutations(pub LanguageId, pub PathBuf, pub Vec<Mutation>);
impl Render for FileMutations {
    fn markdown(&self) -> String {
        format!(
            "# Mutations in {}\n\n{} total\n\n{}",
            self.1.display(),
            self.2.len(),
            fmt_mutation_markdown_table(&self.2),
        )
    }

    fn terse(&self) -> String {
        self.2.iter().map(fmt_mutation_terse).collect::<Vec<_>>().join("\n")
    }

    fn verbose(&self) -> String {
        self.2.iter().map(fmt_mutation_verbose).collect::<Vec<_>>().join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

pub fn find_mutation_by_hash(hash: &str, mutations: Vec<Mutation>) -> Mutation {
    let unvalidated = UnvalidatedHash::new(hash.to_string());
    let hashes: Vec<_> = mutations
        .iter()
        .map(|m| m.hash().expect("hashing should not fail"))
        .collect();
    let matched = unvalidated
        .validate(&hashes)
        .expect("hash resolution failed");
    let matched_bytes = matched.as_bytes();
    mutations
        .into_iter()
        .find(|m| m.hash().unwrap().as_bytes() == matched_bytes)
        .unwrap()
}

pub struct SingleMutation {
    pub state: State,
    pub context: String,
    pub lang: LanguageId,
}

impl Render for SingleMutation {
    fn markdown(&self) -> String {
        let m = self.state.mutation();
        let outcome = if self.state.has_outcome() { "has outcome" } else { "pending" };
        let lang_tag = match self.lang {
            LanguageId::Javascript => "javascript",
            LanguageId::Typescript => "typescript",
        };
        format!(
            "# Mutation\n\n{}\n\nStatus: {}\n\n## Context\n\n```{}\n{}\n```",
            fmt_mutation_markdown_table(std::slice::from_ref(m)),
            outcome,
            lang_tag,
            self.context,
        )
    }

    fn terse(&self) -> String {
        let m = self.state.mutation();
        let outcome = if self.state.has_outcome() { "has outcome" } else { "pending" };
        format!("{} {}", fmt_mutation_terse(m), outcome)
    }

    fn verbose(&self) -> String {
        let m = self.state.mutation();
        let outcome = if self.state.has_outcome() { "has outcome" } else { "pending" };
        format!("{}\nStatus: {}\n\n{}", fmt_mutation_verbose(m), outcome, self.context)
    }

    fn json(&self) -> String {
        facet_json::to_string(&self.state).unwrap()
    }
}

impl Render for Config {
    fn markdown(&self) -> String {
        format!(
            "# Bough Config

```json
{}
```",
            facet_json::to_string(self).unwrap()
        )
    }

    fn terse(&self) -> String {
        facet_json::to_string(self).unwrap()
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Bough Config{RESET}\n\n{}",
            facet_json::to_string(self).unwrap()
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

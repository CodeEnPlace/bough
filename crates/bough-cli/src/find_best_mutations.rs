use std::ops::Deref;
use std::path::PathBuf;

use bough_core::{LanguageId, MutationHash, Session, State};

use crate::config::Config;
use crate::render::{
    HASH, PATH, RESET, STRING, TITLE, Render, fmt_status_colored, mutation_hash,
};

pub struct FindBestMutations(pub Vec<(MutationHash, State, f64)>);

impl FindBestMutations {
    pub fn run(session: impl Deref<Target = Session<Config>>, lang: Option<LanguageId>, file: Option<PathBuf>) -> Box<Self> {
        let results = session.find_best_mutations().expect("find best mutations");
        let filtered: Vec<_> = results
            .into_iter()
            .filter(|(_, state, _)| {
                if let Some(l) = lang {
                    if state.mutation().mutant().lang() != l {
                        return false;
                    }
                }
                if let Some(ref f) = file {
                    if state.mutation().mutant().twig().path() != f.as_path() {
                        return false;
                    }
                }
                true
            })
            .collect();
        Box::new(Self(filtered))
    }
}

impl Render for FindBestMutations {
    fn markdown(&self) -> String {
        let mut out = format!("# Find Best Mutations\n\n{} selected\n\n", self.0.len());
        out.push_str("| # | Score | Hash | File | Location | Kind | Subst |\n");
        out.push_str("|---|-------|------|------|----------|------|-------|\n");
        for (i, (hash, state, score)) in self.0.iter().enumerate() {
            let m = state.mutation();
            let mutant = m.mutant();
            out.push_str(&format!(
                "| {} | {:.2} | `{}` | {} | {}:{} | {:?} | `{}` |\n",
                i + 1,
                score,
                hash,
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                mutant.kind(),
                m.subst(),
            ));
        }
        out
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|(_, state, score)| {
                let m = state.mutation();
                let mutant = m.mutant();
                format!(
                    "{HASH}{}{RESET} {:.2} {} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
                    mutation_hash(m),
                    score,
                    fmt_status_colored(state),
                    mutant.twig().path().display(),
                    mutant.span().start().line() + 1,
                    mutant.span().start().col() + 1,
                    mutant.kind(),
                    m.subst(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let mut out = format!("{TITLE}Find Best Mutations{RESET} ({} selected)\n", self.0.len());
        for (i, (hash, state, score)) in self.0.iter().enumerate() {
            let m = state.mutation();
            let mutant = m.mutant();
            out.push_str(&format!(
                "\n  {HASH}#{}{RESET} score={:.2} {HASH}{}{RESET} {} {PATH}{}:{}:{}{RESET} {:?} → {STRING}{}{RESET}",
                i + 1,
                score,
                hash,
                fmt_status_colored(state),
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                mutant.kind(),
                m.subst(),
            ));
        }
        out
    }

    fn json(&self) -> String {
        let items: Vec<String> = self.0.iter().map(|(hash, state, score)| {
            let m = state.mutation();
            let mutant = m.mutant();
            format!(
                r#"{{"hash":"{}","score":{},"file":"{}","line":{},"col":{},"kind":"{}","subst":"{}"}}"#,
                hash,
                score,
                mutant.twig().path().display(),
                mutant.span().start().line() + 1,
                mutant.span().start().col() + 1,
                format!("{:?}", mutant.kind()),
                m.subst(),
            )
        }).collect();
        format!("[{}]", items.join(","))
    }
}

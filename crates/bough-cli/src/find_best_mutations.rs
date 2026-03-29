use std::ops::Deref;
use std::path::PathBuf;

use bough_core::{LanguageId, MutationHash, Session, State};

use crate::config::Config;
use crate::render::{BOLD, RESET, Render, TITLE};

pub struct FindBestMutations(pub Vec<(MutationHash, State, f64)>);

impl FindBestMutations {
    pub fn run(
        session: impl Deref<Target = Session<Config>>,
        lang: Option<LanguageId>,
        file: Option<PathBuf>,
    ) -> Box<Self> {
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
        let header = "| # | Score | State |\n| --- | --- | --- |";
        let rows = self
            .0
            .iter()
            .enumerate()
            .map(|(i, (_, state, score))| {
                format!("| {} | {:.2} | {} |", i + 1, score, state.markdown())
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}# Best Mutations{RESET}\n\n{} selected\n\n{header}\n{rows}",
            self.0.len(),
        )
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|(_, state, score)| format!("{:.2} {}", score, state.terse()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let list = self
            .0
            .iter()
            .enumerate()
            .map(|(i, (_, state, score))| {
                format!(
                    "  {BOLD}#{}{RESET} score={:.2} {}",
                    i + 1,
                    score,
                    state.verbose(),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}Best Mutations{RESET} ({} selected)\n{list}",
            self.0.len(),
        )
    }

    fn json(&self) -> String {
        let items: Vec<String> = self
            .0
            .iter()
            .map(|(hash, state, score)| {
                format!(
                    r#"{{"hash":"{}","score":{},"state":{}}}"#,
                    hash,
                    score,
                    state.json(),
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_core::*;
    use bough_typed_hash::TypedHashable;

    fn fixture() -> FindBestMutations {
        let mutant = Mutant::new(
            LanguageId::Typescript,
            Twig::new("src/main.ts".into()).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
        );
        let mutation = MutationIter::new(&mutant).next().unwrap();
        let hash = mutation.hash().expect("hash");
        let state = State::new(mutation);
        FindBestMutations(vec![(hash, state, 0.75)])
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# Best Mutations\n\n1 selected"));
        assert!(plain.contains("0.75"));
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(out.contains("0.75"));
        assert_eq!(out.lines().count(), 1);
    }

    #[test]
    fn verbose() {
        let plain = fixture()
            .verbose()
            .replace(TITLE, "")
            .replace(BOLD, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Best Mutations (1 selected)"));
        assert!(plain.contains("score=0.75"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('['));
        assert!(out.contains(r#""score":0.75"#));
    }
}

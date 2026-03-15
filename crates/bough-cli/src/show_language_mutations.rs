use std::ops::DerefMut;

use bough_core::{LanguageId, Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{TITLE, RESET, Render, render_table};

#[derive(Facet)]
pub struct ShowLanguageMutations(pub LanguageId, pub Vec<State>);

impl ShowLanguageMutations {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, lang: LanguageId) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan")
            .into_iter()
            .filter(|m| m.mutant().lang() == lang)
            .collect();
        let states = mutations
            .into_iter()
            .map(|m| {
                let hash = m.hash().expect("hash");
                session
                    .get_state()
                    .get(&hash)
                    .expect("state not found for mutation")
            })
            .collect();
        Box::new(Self(lang, states))
    }
}

impl Render for ShowLanguageMutations {
    fn markdown(&self) -> String {
        let rows: Vec<_> = self.1.iter().map(|s| s.tabular()).collect();
        format!(
            "{TITLE}# {} Mutations{RESET}\n\n{} total\n\n{}",
            self.0.markdown(),
            self.1.len(),
            render_table(&rows),
        )
    }

    fn terse(&self) -> String {
        self.1.iter().map(|s| s.terse()).collect::<Vec<_>>().join("\n")
    }

    fn verbose(&self) -> String {
        let list = self.1.iter().map(|s| s.verbose()).collect::<Vec<_>>().join("\n");
        format!(
            "{TITLE}{} Mutations{RESET} ({} total)\n\n{list}",
            self.0.verbose(),
            self.1.len(),
        )
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_core::*;

    fn make_state() -> State {
        let mutant = Mutant::new(
            LanguageId::Typescript,
            Twig::new("src/main.ts".into()).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
        );
        State::new(MutationIter::new(&mutant).next().unwrap())
    }

    fn fixture() -> ShowLanguageMutations {
        ShowLanguageMutations(LanguageId::Typescript, vec![make_state()])
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown()
            .replace(TITLE, "").replace(crate::render::LANG, "").replace(RESET, "");
        assert!(plain.starts_with("# TypeScript Mutations\n\n1 total"));
    }

    #[test]
    fn terse() {
        assert_eq!(fixture().terse().lines().count(), 1);
    }

    #[test]
    fn verbose() {
        let plain = fixture().verbose()
            .replace(TITLE, "").replace(crate::render::LANG, "").replace(RESET, "");
        assert!(plain.starts_with("TypeScript Mutations (1 total)"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('['));
    }
}



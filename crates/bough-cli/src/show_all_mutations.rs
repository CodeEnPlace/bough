use std::ops::DerefMut;

use bough_core::{Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{TITLE, RESET, Render, render_table};

#[derive(Facet)]
pub struct ShowAllMutations(pub Vec<State>);

impl ShowAllMutations {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan");
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
        Box::new(Self(states))
    }
}

impl Render for ShowAllMutations {
    fn markdown(&self) -> String {
        let rows: Vec<_> = self.0.iter().map(|s| s.tabular()).collect();
        format!(
            "{TITLE}# All Mutations{RESET}\n\n{} total\n\n{}",
            self.0.len(),
            render_table(&rows),
        )
    }

    fn terse(&self) -> String {
        self.0.iter().map(|s| s.terse()).collect::<Vec<_>>().join("\n")
    }

    fn verbose(&self) -> String {
        let list = self.0.iter().map(|s| s.verbose()).collect::<Vec<_>>().join("\n");
        format!(
            "{TITLE}All Mutations{RESET} ({} total)\n\n{list}",
            self.0.len(),
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
        let mutation = MutationIter::new(&mutant).next().unwrap();
        State::new(mutation)
    }

    #[test]
    fn markdown() {
        let sam = ShowAllMutations(vec![make_state()]);
        let plain = sam.markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# All Mutations\n\n1 total\n\n"));
        assert!(plain.contains("src/main.ts"));
    }

    #[test]
    fn terse() {
        let sam = ShowAllMutations(vec![make_state(), make_state()]);
        assert_eq!(sam.terse().lines().count(), 2);
    }

    #[test]
    fn verbose() {
        let sam = ShowAllMutations(vec![make_state()]);
        let plain = sam.verbose().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("All Mutations (1 total)"));
    }

    #[test]
    fn json() {
        let sam = ShowAllMutations(vec![make_state()]);
        let out = sam.json();
        assert!(out.starts_with('['));
        assert!(out.ends_with(']'));
    }
}



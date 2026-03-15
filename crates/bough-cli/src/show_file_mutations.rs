use std::ops::DerefMut;
use std::path::PathBuf;

use bough_core::{LanguageId, Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, TITLE, RESET, Render};

#[derive(Facet)]
pub struct ShowFileMutations(pub LanguageId, pub PathBuf, pub Vec<State>);

impl ShowFileMutations {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, lang: LanguageId, file: PathBuf) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan")
            .into_iter()
            .filter(|m| {
                m.mutant().lang() == lang && m.mutant().twig().path() == file.as_path()
            })
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
        Box::new(Self(lang, file, states))
    }
}

impl Render for ShowFileMutations {
    fn markdown(&self) -> String {
        format!(
            "{TITLE}# Mutations in {PATH}{}{RESET}\n\n{} total\n\n{}",
            self.1.display(),
            self.2.len(),
            self.2.markdown(),
        )
    }

    fn terse(&self) -> String {
        self.2.terse()
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Mutations in{RESET} {PATH}{}{RESET} ({} total)\n\n{}",
            self.1.display(),
            self.2.len(),
            self.2.verbose(),
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

    fn fixture() -> ShowFileMutations {
        ShowFileMutations(
            LanguageId::Typescript,
            PathBuf::from("src/main.ts"),
            vec![make_state()],
        )
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown()
            .replace(TITLE, "").replace(PATH, "").replace(RESET, "");
        assert!(plain.starts_with("# Mutations in src/main.ts\n\n1 total"));
    }

    #[test]
    fn terse() {
        let f = fixture();
        assert_eq!(f.terse().lines().count(), 1);
    }

    #[test]
    fn verbose() {
        let plain = fixture().verbose()
            .replace(TITLE, "").replace(PATH, "").replace(RESET, "");
        assert!(plain.starts_with("Mutations in src/main.ts (1 total)"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('['));
    }
}



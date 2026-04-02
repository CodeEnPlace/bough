use std::ops::DerefMut;
use std::path::PathBuf;

use bough_core::{LanguageId};
use bough_lib::{Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, RESET, Render, TITLE, render_table};

#[derive(Facet)]
pub struct ShowFileMutations(pub LanguageId, pub PathBuf, pub Vec<State>);

impl ShowFileMutations {
    pub fn run(
        mut session: impl DerefMut<Target = Session<Config>>,
        lang: LanguageId,
        file: PathBuf,
    ) -> Box<Self> {
        session.tend_add_missing_states().expect("tend states");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan")
            .into_iter()
            .filter(|m| m.mutant().lang() == lang && m.mutant().twig().path() == file.as_path())
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
        let rows: Vec<_> = self.2.iter().map(|s| s.tabular()).collect();
        format!(
            "{TITLE}# Mutations in {PATH}{}{RESET}\n\n{} total\n\n{}",
            self.1.display(),
            self.2.len(),
            render_table(&rows),
        )
    }

    fn terse(&self) -> String {
        self.2
            .iter()
            .map(|s| s.terse())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        let list = self
            .2
            .iter()
            .map(|s| s.verbose())
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}Mutations in{RESET} {PATH}{}{RESET} ({} total)\n\n{list}",
            self.1.display(),
            self.2.len(),
        )
    }

    fn json(&self) -> String {
        let states: Vec<String> = self.2.iter().map(|s| s.json()).collect();
        format!(
            r#"{{"lang":{},"file":"{}","states":[{}]}}"#,
            self.0.json(),
            self.1.display(),
            states.join(","),
        )
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
        State::new(MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver).next().unwrap())
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
        let plain = fixture()
            .markdown()
            .replace(TITLE, "")
            .replace(PATH, "")
            .replace(RESET, "");
        assert!(plain.starts_with("# Mutations in src/main.ts\n\n1 total"));
    }

    #[test]
    fn terse() {
        let f = fixture();
        assert_eq!(f.terse().lines().count(), 1);
    }

    #[test]
    fn verbose() {
        let plain = fixture()
            .verbose()
            .replace(TITLE, "")
            .replace(PATH, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Mutations in src/main.ts (1 total)"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('{'));
        assert!(out.contains(r#""lang":"ts""#));
        assert!(out.contains(r#""file":"src/main.ts""#));
    }
}

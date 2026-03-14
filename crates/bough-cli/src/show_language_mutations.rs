use bough_core::{LanguageId, Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{Render, fmt_mutation_markdown_table, fmt_mutation_terse, fmt_mutation_verbose};

#[derive(Facet)]
pub struct ShowLanguageMutations(pub LanguageId, pub Vec<State>);

impl ShowLanguageMutations {
    pub fn run(config: Config, lang: LanguageId) -> Box<Self> {
        let mut session = Session::new(config).expect("session creation");
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
        let mutations: Vec<_> = self.1.iter().map(|s| s.mutation().clone()).collect();
        format!(
            "# {:?} Mutations\n\n{} total\n\n{}",
            self.0,
            self.1.len(),
            fmt_mutation_markdown_table(&mutations),
        )
    }

    fn terse(&self) -> String {
        self.1
            .iter()
            .map(fmt_mutation_terse)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        self.1
            .iter()
            .map(fmt_mutation_verbose)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

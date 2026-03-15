use std::ops::DerefMut;

use bough_core::{Session, State};
use bough_typed_hash::TypedHashable;
use facet::Facet;

use crate::config::Config;
use crate::render::{Render, fmt_mutation_markdown_table, fmt_mutation_terse, fmt_mutation_verbose};

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
        let mutations: Vec<_> = self.0.iter().map(|s| s.mutation().clone()).collect();
        format!(
            "# All Mutations\n\n{} total\n\n{}",
            self.0.len(),
            fmt_mutation_markdown_table(&mutations),
        )
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(fmt_mutation_terse)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(fmt_mutation_verbose)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

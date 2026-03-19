#![allow(dead_code)]

mod base;
mod facet_disk_store;
mod file;
mod language;
pub mod mutant;
mod mutation;
mod mutation_score;
mod phase;
mod session;
mod state;
mod test_id;
mod twig;
mod workspace;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    facet::Facet,
    bough_typed_hash::HashInto,
)]
#[facet(rename_all = "lowercase")]
#[repr(u8)]
pub enum LanguageId {
    #[facet(rename = "js")]
    Javascript,
    #[facet(rename = "ts")]
    Typescript,
    #[facet(rename = "py")]
    Python,
}

impl LanguageId {
    pub const ALL: &[LanguageId] = &[
        LanguageId::Javascript,
        LanguageId::Typescript,
        LanguageId::Python,
    ];

    pub fn substitutions(&self, kind: &MutantKind) -> Vec<String> {
        language::driver_for_lang(*self).substitutions(kind)
    }

    pub fn slug(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "js",
            LanguageId::Typescript => "ts",
            LanguageId::Python => "py",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "JavaScript",
            LanguageId::Typescript => "TypeScript",
            LanguageId::Python => "Python",
        }
    }

    pub fn corpus_dir_name(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "javascript",
            LanguageId::Typescript => "typescript",
            LanguageId::Python => "python",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        self.slug()
    }
}

pub use base::Base;
pub use file::File;
pub use file::Twig;
pub use mutant::{Mutant, MutantKind, Point, Span};
pub use mutation::Mutation;
pub use mutation::MutationHash;
pub use mutation::MutationIter;
pub use mutation_score::Factor;
pub use phase::Error as PhaseError;
pub use phase::PhaseOutcome;
pub use session::Config;
pub use session::Session;
pub use state::{State, Status};
pub use twig::TwigsIterBuilder;
pub use workspace::{Error as WorkspaceError, Workspace, WorkspaceId};

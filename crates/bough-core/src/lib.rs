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

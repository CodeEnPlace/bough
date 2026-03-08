#![allow(dead_code)]

mod base;
mod facet_disk_store;
mod file;
mod language;
mod mutant;
mod mutation;
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

pub use file::File;
pub use mutation::Mutation;
pub use mutation::MutationHash;
pub use session::Config;
pub use session::Session;
pub use state::State;
pub use phase::PhaseOutcome;
pub use workspace::{Workspace, WorkspaceId};

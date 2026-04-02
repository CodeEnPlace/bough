#![allow(dead_code)]

mod base;
mod facet_disk_store;
pub mod language;
mod mutation_score;
mod phase;
mod session;
mod state;
mod test_id;
mod uncovered;
mod workspace;

pub use base::Base;
pub use mutation_score::Factor;
pub use mutation_score::MutationScorer;
pub use phase::Error as PhaseError;
pub use phase::PhaseOutcome;
pub use session::Config;
pub use session::Session;
pub use state::{State, Status};
pub use workspace::{Error as WorkspaceError, Workspace, WorkspaceId};

#[cfg(test)]
mod core_tests;

pub use bough_core;

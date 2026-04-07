#![allow(dead_code)]

mod base_ops;
mod facet_disk_store;
pub mod language;
mod mutation_score;
mod phase;
mod session;
mod state;
mod test_id;
mod uncovered;
mod workspace;

pub use base_ops::{mutants, mutations, run_init_in_base, run_reset_in_base, run_test_in_base};
pub use bough_dirs::Base;
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
pub use bough_dirs;

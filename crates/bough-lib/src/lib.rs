#![allow(dead_code)]

mod dirs_ops;
mod facet_disk_store;
pub mod language;
mod mutation_score;
mod phase;
mod session;
mod state;
mod test_id;
mod uncovered;

pub use dirs_ops::{
    mutants, mutations, run_init_in_base, run_init_in_workspace, run_reset_in_base,
    run_reset_in_workspace, run_test_in_base, run_test_in_workspace,
};
pub use mutation_score::MutationScorer;
pub use phase::Error as PhaseError;
pub use phase::PhaseOutcome;
pub use session::Session;
pub use state::{State, Status};

#[cfg(test)]
mod core_tests;

pub use bough_core;
pub use bough_dirs;

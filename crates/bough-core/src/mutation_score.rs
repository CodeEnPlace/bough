use std::sync::Arc;

use facet::Facet;

use crate::{Mutation, base::Base};

#[derive(Facet, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Factor {
    /// How many authors have touched this file
    FileAuthorCount,
    /// Severity of the mutation operator (e.g. removing a null check vs flipping a comparator)
    MutationSeverity,
    /// How many mutations have a span that includes this mutation
    OuterMissedMutations,
    /// How many other surviving mutants exist in the same function
    SiblingMissedMutations,
    /// How many distinct mutation operator types survive in the same function
    SiblingOperatorDiversity,
    /// How deep into the tree-sitteer node graph is this mutation
    TSNodeDepth,
    /// How many times has this file been modified in version control
    VcsFileChurn,
    /// How recently was this line touched
    VcsLineBlameRecency,
}

struct OpaqueScore(u64);

pub struct MutationScorer {
    factor: Factor,
    base: Arc<Base>,
}

pub struct MutationScoreViewer {
    min: u64,
    max: u64,
}

impl MutationScorer {
    pub fn new(base: Arc<Base>, factor: Factor) -> Self {
        Self { base, factor }
    }

    pub fn score(&mut self, mutation: Mutation) -> OpaqueScore {
        todo!()
    }

    pub fn view(self) -> MutationScoreViewer {
        todo!()
    }
}

impl MutationScoreViewer {
    fn view(&self, os: OpaqueScore) -> f64 {
        todo!()
    }
}

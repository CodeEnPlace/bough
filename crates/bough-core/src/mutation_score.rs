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
    EncompasingMissedMutationsCount,
    /// How many other surviving mutants exist in the same function
    SiblingMissedMutations,
    /// How many distinct mutation operator types survive in the same function
    SiblingOperatorDiversity,
    /// How deep into the tree-sitter node graph is this mutation
    TSNodeDepth,
    /// How many times has this file been modified in version control
    VcsFileChurn,
    /// How recently was this line touched
    VcsLineBlameRecency,
}

pub struct OpaqueScore(u64);

pub struct MutationScorer {
    factor: Factor,
    base: Arc<Base>,
    min: u64,
    max: u64,
}

pub struct MutationScoreViewer {
    min: u64,
    max: u64,
}

impl MutationScorer {
    pub fn new(base: Arc<Base>, factor: Factor) -> Self {
        Self {
            base,
            factor,
            min: u64::MAX,
            max: u64::MIN,
        }
    }

    pub fn score(&mut self, mutation: Mutation) -> OpaqueScore {
        match &self.factor {
            &Factor::TSNodeDepth => todo!(),

            &Factor::EncompasingMissedMutationsCount => todo!(),

            &Factor::FileAuthorCount => todo!(),
            &Factor::MutationSeverity => todo!(),
            &Factor::SiblingMissedMutations => todo!(),
            &Factor::SiblingOperatorDiversity => todo!(),
            &Factor::VcsFileChurn => todo!(),
            &Factor::VcsLineBlameRecency => todo!(),
        }
    }

    pub fn into_viewer(self) -> MutationScoreViewer {
        MutationScoreViewer {
            min: self.min,
            max: self.max,
        }
    }
}

impl MutationScoreViewer {
    pub fn normalize(&self, score: OpaqueScore) -> f64 {
        assert!(
            score.0 >= self.min && score.0 <= self.max,
            "OpaqueScore {} out of range [{}, {}]",
            score.0,
            self.min,
            self.max,
        );
        if self.min == self.max {
            return 0.0;
        }
        (score.0 - self.min) as f64 / (self.max - self.min) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod scorer_and_viewer {
        use super::*;

        fn make_scorer(min: u64, max: u64) -> MutationScorer {
            let base = Arc::new(crate::base::Base::new(
                std::env::temp_dir(),
                crate::twig::TwigsIterBuilder::new(),
            ).unwrap());
            MutationScorer { base, factor: Factor::TSNodeDepth, min, max }
        }

        #[test]
        fn viewer_normalizes_min_to_zero() {
            let viewer = make_scorer(10, 20).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(10)), 0.0);
        }

        #[test]
        fn viewer_normalizes_max_to_one() {
            let viewer = make_scorer(10, 20).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(20)), 1.0);
        }

        #[test]
        fn viewer_normalizes_midpoint() {
            let viewer = make_scorer(0, 100).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(50)), 0.5);
        }

        #[test]
        fn viewer_normalizes_single_value_to_zero() {
            let viewer = make_scorer(42, 42).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(42)), 0.0);
        }

        #[test]
        #[should_panic]
        fn viewer_panics_below_min() {
            let viewer = make_scorer(10, 20).into_viewer();
            viewer.normalize(OpaqueScore(9));
        }

        #[test]
        #[should_panic]
        fn viewer_panics_above_max() {
            let viewer = make_scorer(10, 20).into_viewer();
            viewer.normalize(OpaqueScore(21));
        }

        #[test]
        fn viewer_normalizes_large_range() {
            let viewer = make_scorer(0, u64::MAX).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(0)), 0.0);
            assert_eq!(viewer.normalize(OpaqueScore(u64::MAX)), 1.0);
            assert_eq!(viewer.normalize(OpaqueScore(u64::MAX / 2)), 0.5);
        }

        #[test]
        fn viewer_normalizes_narrow_range() {
            let viewer = make_scorer(1_000_000, 1_000_001).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(1_000_000)), 0.0);
            assert_eq!(viewer.normalize(OpaqueScore(1_000_001)), 1.0);
        }

        #[test]
        fn viewer_normalizes_high_values_narrow_range() {
            let viewer = make_scorer(u64::MAX - 10, u64::MAX).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(u64::MAX - 10)), 0.0);
            assert_eq!(viewer.normalize(OpaqueScore(u64::MAX)), 1.0);
            assert_eq!(viewer.normalize(OpaqueScore(u64::MAX - 5)), 0.5);
        }

        #[test]
        fn viewer_normalizes_quarter_points() {
            let viewer = make_scorer(0, 1_000_000_000).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(250_000_000)), 0.25);
            assert_eq!(viewer.normalize(OpaqueScore(750_000_000)), 0.75);
        }

        #[test]
        fn viewer_inherits_min_max_from_scorer() {
            let viewer = make_scorer(5, 15).into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(5)), 0.0);
            assert_eq!(viewer.normalize(OpaqueScore(15)), 1.0);
            assert_eq!(viewer.normalize(OpaqueScore(10)), 0.5);
        }
    }

    mod ts_node_depth {}
    mod encompasing_missed_mutations_count {}
    mod file_author_count {}
    mod mutation_severity {}
    mod sibling_missed_mutations {}
    mod sibling_operator_diversity {}
    mod vcs_file_churn {}
    mod vcs_line_blame_recency {}
}

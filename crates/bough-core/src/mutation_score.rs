use std::sync::Arc;

use facet::Facet;

use crate::{Mutation, base::Base};
use crate::language::driver_for_lang;

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

    pub fn score(&mut self, mutation: Mutation, states: &[crate::state::State]) -> OpaqueScore {
        let value = match &self.factor {
            Factor::TSNodeDepth => self.score_ts_node_depth(&mutation),

            Factor::EncompasingMissedMutationsCount => self.score_encompassing_missed(&mutation, states),
            Factor::FileAuthorCount => todo!(),
            Factor::MutationSeverity => todo!(),
            Factor::SiblingMissedMutations => todo!(),
            Factor::SiblingOperatorDiversity => todo!(),
            Factor::VcsFileChurn => todo!(),
            Factor::VcsLineBlameRecency => todo!(),
        };
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        OpaqueScore(value)
    }

    fn score_ts_node_depth(&self, mutation: &Mutation) -> u64 {
        let mutant = mutation.mutant();
        let file_path = crate::file::File::new(self.base.as_ref(), mutant.twig()).resolve();
        let content = std::fs::read(&file_path).expect("source file should be readable");

        let driver = driver_for_lang(mutant.lang());
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&driver.ts_language())
            .expect("language grammar should load");
        let tree = parser.parse(&content, None).expect("parse should succeed");

        let node = tree
            .root_node()
            .descendant_for_byte_range(mutant.span().start().byte(), mutant.span().end().byte())
            .unwrap_or(tree.root_node());

        let mut depth = 0u64;
        let mut current = node;
        while let Some(parent) = current.parent() {
            depth += 1;
            current = parent;
        }
        depth
    }

    fn score_encompassing_missed(&self, mutation: &Mutation, states: &[crate::state::State]) -> u64 {
        let target = mutation.mutant();
        states
            .iter()
            .filter(|s| matches!(s.status(), Some(crate::state::Status::Missed)))
            .filter(|s| s.mutation().mutant() != target)
            .filter(|s| s.mutation().mutant().encompasses(target).unwrap_or(false))
            .count() as u64
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

    mod ts_node_depth {
        use super::*;
        use crate::file::Twig;
        use crate::mutant::{MutantKind, Mutant, Point, Span, BinaryOpMutationKind, TwigMutantsIter};
        use crate::mutation::MutationIter;
        use crate::twig::TwigsIterBuilder;
        use crate::LanguageId;
        use std::path::PathBuf;

        fn make_js_base(content: &str) -> (tempfile::TempDir, crate::base::Base) {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("src")).unwrap();
            std::fs::write(dir.path().join("src/a.js"), content).unwrap();
            let base = crate::base::Base::new(
                dir.path().to_path_buf(),
                TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
            ).unwrap();
            (dir, base)
        }

        fn score_all(base: &crate::base::Base) -> Vec<(MutantKind, u64)> {
            let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
            let mut scorer = MutationScorer::new(Arc::new(base.clone()), Factor::TSNodeDepth);
            TwigMutantsIter::new(LanguageId::Javascript, base, &twig)
                .unwrap()
                .flat_map(|bm| {
                    let mutant = bm.into_mutant();
                    MutationIter::new(&mutant).collect::<Vec<_>>()
                })
                .map(|mutation| {
                    let kind = mutation.mutant().kind().clone();
                    let score = scorer.score(mutation, &[]);
                    (kind, score.0)
                })
                .collect()
        }

        #[test]
        fn top_level_statement_block_is_shallow() {
            let (_dir, base) = make_js_base("function foo() { return 1; }");
            let scores = score_all(&base);
            let block_scores: Vec<_> = scores.iter()
                .filter(|(k, _)| matches!(k, MutantKind::StatementBlock))
                .collect();
            assert_eq!(block_scores.len(), 1);
            let depth = block_scores[0].1;
            assert!(depth <= 3, "top-level block depth {depth} should be shallow");
        }

        #[test]
        fn nested_mutation_is_deeper_than_outer() {
            let js = "function foo() { if (x) { return a + b; } }";
            let (_dir, base) = make_js_base(js);
            let scores = score_all(&base);
            let block_depth = scores.iter()
                .filter(|(k, _)| matches!(k, MutantKind::StatementBlock))
                .map(|(_, d)| *d)
                .min().unwrap();
            let add_depth = scores.iter()
                .filter(|(k, _)| matches!(k, MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
                .map(|(_, d)| *d)
                .next().unwrap();
            assert!(add_depth > block_depth, "add depth {add_depth} should be deeper than block depth {block_depth}");
        }

        #[test]
        fn scorer_updates_min_max() {
            let js = "function foo() { if (x) { return a + b; } }";
            let (_dir, base) = make_js_base(js);
            let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
            let mut scorer = MutationScorer::new(Arc::new(base.clone()), Factor::TSNodeDepth);
            let mutations: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
                .unwrap()
                .flat_map(|bm| MutationIter::new(&bm.into_mutant()).collect::<Vec<_>>())
                .collect();
            let scores: Vec<_> = mutations.into_iter()
                .map(|m| scorer.score(m, &[]))
                .collect();
            let viewer = scorer.into_viewer();
            let min_score = scores.iter().map(|s| s.0).min().unwrap();
            let max_score = scores.iter().map(|s| s.0).max().unwrap();
            assert_eq!(viewer.normalize(OpaqueScore(min_score)), 0.0);
            assert_eq!(viewer.normalize(OpaqueScore(max_score)), 1.0);
        }
    }
    mod encompasing_missed_mutations_count {
        use super::*;
        use crate::file::Twig;
        use crate::mutant::{MutantKind, BinaryOpMutationKind, TwigMutantsIter};
        use crate::mutation::{Mutation, MutationIter};
        use crate::state::{State, Status};
        use crate::twig::TwigsIterBuilder;
        use crate::LanguageId;
        use std::path::PathBuf;

        fn make_js_base(content: &str) -> (tempfile::TempDir, crate::base::Base) {
            let dir = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(dir.path().join("src")).unwrap();
            std::fs::write(dir.path().join("src/a.js"), content).unwrap();
            let base = crate::base::Base::new(
                dir.path().to_path_buf(),
                TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
            ).unwrap();
            (dir, base)
        }

        fn all_mutations(base: &crate::base::Base) -> Vec<Mutation> {
            let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
            TwigMutantsIter::new(LanguageId::Javascript, base, &twig)
                .unwrap()
                .flat_map(|bm| MutationIter::new(&bm.into_mutant()).collect::<Vec<_>>())
                .collect()
        }

        fn first_mutation_of_kind(mutations: &[Mutation], kind: &MutantKind) -> Mutation {
            mutations.iter()
                .find(|m| m.mutant().kind() == kind)
                .unwrap_or_else(|| panic!("no mutation of kind {kind:?}"))
                .clone()
        }

        fn make_missed_state(mutation: Mutation) -> State {
            let mut state = State::new(mutation);
            state.set_outcome(Status::Missed);
            state
        }

        fn make_caught_state(mutation: Mutation) -> State {
            let mut state = State::new(mutation);
            state.set_outcome(Status::Caught);
            state
        }

        fn make_scorer(base: &crate::base::Base) -> MutationScorer {
            MutationScorer {
                base: Arc::new(base.clone()),
                factor: Factor::EncompasingMissedMutationsCount,
                min: u64::MAX,
                max: u64::MIN,
            }
        }

        // if (x) { y() + z() }
        // Condition on x: effect_span covers entire if statement
        // BinaryOp +: effect_span covers y() + z()
        // Condition encompasses +, but + does not encompass condition

        #[test]
        fn inner_mutation_scores_higher_than_outer() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![
                make_missed_state(condition.clone()),
                make_missed_state(binop.clone()),
            ];

            let mut scorer = make_scorer(&base);
            let binop_score = scorer.score(binop, &states);
            let condition_score = scorer.score(condition, &states);

            assert!(binop_score.0 > condition_score.0,
                "binop score {} should be higher than condition score {}", binop_score.0, condition_score.0);
        }

        #[test]
        fn condition_not_encompassed_by_binop_scores_zero() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![make_missed_state(binop)];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(condition, &states);
            assert_eq!(score.0, 0);
        }

        #[test]
        fn binop_encompassed_by_condition_scores_one() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![make_missed_state(condition)];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(binop, &states);
            assert_eq!(score.0, 1);
        }

        #[test]
        fn caught_mutations_are_not_counted() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![make_caught_state(condition)];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(binop, &states);
            assert_eq!(score.0, 0);
        }

        #[test]
        fn does_not_count_self() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);

            let states = vec![make_missed_state(condition.clone())];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(condition, &states);
            assert_eq!(score.0, 0);
        }

        #[test]
        fn multiple_encompassing_missed_mutations() {
            // Both the condition and the statement block encompass the binop
            let (_dir, base) = make_js_base("if (x) { return a + b; }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let block = first_mutation_of_kind(&mutations, &MutantKind::StatementBlock);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![
                make_missed_state(condition),
                make_missed_state(block),
            ];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(binop, &states);
            assert_eq!(score.0, 2);
        }

        // Two disjoint functions: mutations in one should not encompass mutations in the other
        #[test]
        fn disjoint_missed_mutations_score_zero() {
            let (_dir, base) = make_js_base(
                "function foo() { return a + b; }\nfunction bar() { return c - d; }"
            );
            let mutations = all_mutations(&base);
            let add = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));
            let sub = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Sub));

            let states = vec![make_missed_state(sub)];

            let mut scorer = make_scorer(&base);
            let score = scorer.score(add, &states);
            assert_eq!(score.0, 0);
        }

        #[test]
        fn updates_min_max() {
            let (_dir, base) = make_js_base("if (x) { y() + z() }");
            let mutations = all_mutations(&base);
            let condition = first_mutation_of_kind(&mutations, &MutantKind::Condition);
            let binop = first_mutation_of_kind(&mutations, &MutantKind::BinaryOp(BinaryOpMutationKind::Add));

            let states = vec![
                make_missed_state(condition.clone()),
                make_missed_state(binop.clone()),
            ];

            let mut scorer = make_scorer(&base);
            let s1 = scorer.score(binop, &states);
            let s2 = scorer.score(condition, &states);
            let viewer = scorer.into_viewer();
            assert_eq!(viewer.normalize(OpaqueScore(s1.0)), 1.0);
            assert_eq!(viewer.normalize(OpaqueScore(s2.0)), 0.0);
        }
    }
    mod file_author_count {}
    mod mutation_severity {}
    mod sibling_missed_mutations {}
    mod sibling_operator_diversity {}
    mod vcs_file_churn {}
    mod vcs_line_blame_recency {}
}

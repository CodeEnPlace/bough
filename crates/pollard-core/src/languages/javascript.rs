use crate::{Language, MutationKind};
use super::common::binary_op_substitutions;

pub struct JavaScript;

#[derive(Debug, PartialEq)]
pub enum JsMutationKind {
    StatementBlock,
    BinaryOp,
}

impl From<JsMutationKind> for MutationKind {
    fn from(k: JsMutationKind) -> Self {
        match k {
            JsMutationKind::StatementBlock => MutationKind::StatementBlock,
            JsMutationKind::BinaryOp => MutationKind::BinaryOp,
        }
    }
}

impl Language for JavaScript {
    type Kind = JsMutationKind;

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn mutation_kind_for_node(node_kind: &str) -> Option<JsMutationKind> {
        match node_kind {
            "statement_block" => Some(JsMutationKind::StatementBlock),
            "binary_expression" => Some(JsMutationKind::BinaryOp),
            _ => None,
        }
    }

    fn generate_substitutions(kind: &JsMutationKind, span_text: &str) -> Vec<String> {
        match kind {
            JsMutationKind::StatementBlock => vec!["{}".to_string()],
            JsMutationKind::BinaryOp => binary_op_substitutions(span_text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceFile, Span, find_mutation_points};
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        SourceFile { path: PathBuf::from("test.js"), content: content.to_string() }
    }

    #[test]
    fn finds_function_body() {
        let f = file("function foo() { return 1; }");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].span, Span { start: 15, end: 28 });
        assert_eq!(points[0].kind, JsMutationKind::StatementBlock);
    }

    #[test]
    fn finds_nested_blocks() {
        let f = file("function foo() { if (x) { return 1; } }");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn kind_converts_to_unified() {
        let unified: MutationKind = JsMutationKind::StatementBlock.into();
        assert_eq!(unified, MutationKind::StatementBlock);
    }

    #[test]
    fn finds_addition() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp);
    }

    #[test]
    fn finds_multiplication() {
        let f = file("const x = a * b;");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp);
    }

    #[test]
    fn finds_logical_and() {
        let f = file("const x = a && b;");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp);
    }
}

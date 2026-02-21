use crate::{Language, MutationKind};
use super::common::binary_op_substitutions;

pub struct TypeScript;

#[derive(Debug, PartialEq)]
pub enum TsMutationKind {
    StatementBlock,
    BinaryOp,
}

impl From<TsMutationKind> for MutationKind {
    fn from(k: TsMutationKind) -> Self {
        match k {
            TsMutationKind::StatementBlock => MutationKind::StatementBlock,
            TsMutationKind::BinaryOp => MutationKind::BinaryOp,
        }
    }
}

impl Language for TypeScript {
    type Kind = TsMutationKind;

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn mutation_kind_for_node(node_kind: &str) -> Option<TsMutationKind> {
        match node_kind {
            "statement_block" => Some(TsMutationKind::StatementBlock),
            "binary_expression" => Some(TsMutationKind::BinaryOp),
            _ => None,
        }
    }

    fn generate_substitutions(kind: &TsMutationKind, span_text: &str) -> Vec<String> {
        match kind {
            TsMutationKind::StatementBlock => vec!["{}".to_string()],
            TsMutationKind::BinaryOp => binary_op_substitutions(span_text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SourceFile, find_mutation_points};
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        SourceFile { path: PathBuf::from("test.ts"), content: content.to_string() }
    }

    #[test]
    fn finds_function_body() {
        let f = file("function foo(): number { return 1; }");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::StatementBlock);
    }

    #[test]
    fn finds_nested_blocks() {
        let f = file("function foo(): void { if (x) { return; } }");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 2);
    }

    #[test]
    fn kind_converts_to_unified() {
        let unified: MutationKind = TsMutationKind::StatementBlock.into();
        assert_eq!(unified, MutationKind::StatementBlock);
    }

    #[test]
    fn finds_addition() {
        let f = file("const x: number = a + b;");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp);
    }

    #[test]
    fn finds_multiplication() {
        let f = file("const x: number = a * b;");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp);
    }

    #[test]
    fn finds_logical_and() {
        let f = file("const x: boolean = a && b;");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp);
    }
}

use crate::{BinaryOpKind, Language, MutationKind, SourceFile, Span};
use bough_sha::ShaHashable;
use serde::{Deserialize, Serialize};

pub struct TypeScript;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ShaHashable)]
pub enum TsMutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
}

impl From<TsMutationKind> for MutationKind {
    fn from(k: TsMutationKind) -> Self {
        match k {
            TsMutationKind::StatementBlock => MutationKind::StatementBlock,
            TsMutationKind::BinaryOp(op) => MutationKind::BinaryOp(op),
        }
    }
}

impl Language for TypeScript {
    type Kind = TsMutationKind;

    fn code_tag() -> &'static str {
        "typescript"
    }

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn mutation_kind_for_node(
        node: tree_sitter::Node<'_>,
        content: &[u8],
        file: &SourceFile,
    ) -> Option<(TsMutationKind, Span)> {
        match node.kind() {
            "statement_block" => {
                Some((TsMutationKind::StatementBlock, Span::from_node(file, node)))
            }
            "binary_expression" => {
                let op_node = node.child(1)?;
                let op = match op_node.utf8_text(content).ok()? {
                    "+" => BinaryOpKind::Add,
                    "-" => BinaryOpKind::Sub,
                    "*" => BinaryOpKind::Mul,
                    "/" => BinaryOpKind::Div,
                    "&&" => BinaryOpKind::And,
                    "||" => BinaryOpKind::Or,
                    "===" => BinaryOpKind::StrictEq,
                    "!==" => BinaryOpKind::StrictNeq,
                    "==" => BinaryOpKind::Eq,
                    "!=" => BinaryOpKind::Neq,
                    "<" => BinaryOpKind::Lt,
                    "<=" => BinaryOpKind::Lte,
                    ">" => BinaryOpKind::Gt,
                    ">=" => BinaryOpKind::Gte,
                    _ => return None,
                };
                Some((TsMutationKind::BinaryOp(op), Span::from_node(file, op_node)))
            }
            _ => None,
        }
    }

    fn substitutions_for_kind(kind: &TsMutationKind) -> Vec<String> {
        use BinaryOpKind::*;
        let replacements: &[&str] = match kind {
            TsMutationKind::StatementBlock => &["{}"],
            TsMutationKind::BinaryOp(Add) => &["-", "*", "/"],
            TsMutationKind::BinaryOp(Sub) => &["+", "*", "/"],
            TsMutationKind::BinaryOp(Mul) => &["+", "-", "/"],
            TsMutationKind::BinaryOp(Div) => &["+", "-", "*"],
            TsMutationKind::BinaryOp(And) => &["||"],
            TsMutationKind::BinaryOp(Or) => &["&&"],
            TsMutationKind::BinaryOp(StrictEq) => &["!=="],
            TsMutationKind::BinaryOp(StrictNeq) => &["==="],
            TsMutationKind::BinaryOp(Eq) => &["!="],
            TsMutationKind::BinaryOp(Neq) => &["=="],
            TsMutationKind::BinaryOp(Lt) => &[">", "<=", ">="],
            TsMutationKind::BinaryOp(Lte) => &["<", ">", ">="],
            TsMutationKind::BinaryOp(Gt) => &["<", "<=", ">="],
            TsMutationKind::BinaryOp(Gte) => &[">", "<", "<="],
        };
        replacements.iter().map(|r| r.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOpKind, find_mutants};
    use std::path::PathBuf;

    fn src(content: &str) -> (SourceFile, String) {
        let file = SourceFile::from_content(PathBuf::from("test.ts"), content);
        (file, content.to_string())
    }

    #[test]
    fn finds_function_body() {
        let (f, content) = src("function foo(): number { return 1; }");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, TsMutationKind::StatementBlock);
    }

    #[test]
    fn finds_nested_blocks() {
        let (f, content) = src("function foo(): void { if (x) { return; } }");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        assert_eq!(mutants.len(), 2);
    }

    #[test]
    fn kind_converts_to_unified() {
        let unified: MutationKind = TsMutationKind::StatementBlock.into();
        assert_eq!(unified, MutationKind::StatementBlock);
    }

    #[test]
    fn finds_addition() {
        let (f, content) = src("const x: number = a + b;");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::Add));
    }

    #[test]
    fn finds_multiplication() {
        let (f, content) = src("const x: number = a * b;");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::Mul));
    }

    #[test]
    fn finds_logical_and() {
        let (f, content) = src("const x: boolean = a && b;");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::And));
    }
}

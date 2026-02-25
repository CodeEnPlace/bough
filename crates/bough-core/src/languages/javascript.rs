use crate::{BinaryOpKind, Language, MutationKind, SourceFile, Span};
use bough_sha::ShaHashable;
use serde::{Deserialize, Serialize};

pub struct JavaScript;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ShaHashable)]
pub enum JsMutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
}

impl From<JsMutationKind> for MutationKind {
    fn from(k: JsMutationKind) -> Self {
        match k {
            JsMutationKind::StatementBlock => MutationKind::StatementBlock,
            JsMutationKind::BinaryOp(op) => MutationKind::BinaryOp(op),
        }
    }
}

impl Language for JavaScript {
    type Kind = JsMutationKind;

    fn code_tag() -> &'static str {
        "javascript"
    }

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn mutation_kind_for_node(
        node: tree_sitter::Node<'_>,
        content: &[u8],
        file: &SourceFile,
    ) -> Option<(JsMutationKind, Span)> {
        match node.kind() {
            "statement_block" => {
                Some((JsMutationKind::StatementBlock, Span::from_node(file, node)))
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
                Some((JsMutationKind::BinaryOp(op), Span::from_node(file, op_node)))
            }
            _ => None,
        }
    }

    fn substitutions_for_kind(kind: &JsMutationKind) -> Vec<String> {
        use BinaryOpKind::*;
        let replacements: &[&str] = match kind {
            JsMutationKind::StatementBlock => &["{}"],
            JsMutationKind::BinaryOp(Add) => &["-", "*", "/"],
            JsMutationKind::BinaryOp(Sub) => &["+", "*", "/"],
            JsMutationKind::BinaryOp(Mul) => &["+", "-", "/"],
            JsMutationKind::BinaryOp(Div) => &["+", "-", "*"],
            JsMutationKind::BinaryOp(And) => &["||"],
            JsMutationKind::BinaryOp(Or) => &["&&"],
            JsMutationKind::BinaryOp(StrictEq) => &["!=="],
            JsMutationKind::BinaryOp(StrictNeq) => &["==="],
            JsMutationKind::BinaryOp(Eq) => &["!="],
            JsMutationKind::BinaryOp(Neq) => &["=="],
            JsMutationKind::BinaryOp(Lt) => &[">", "<=", ">="],
            JsMutationKind::BinaryOp(Lte) => &["<", ">", ">="],
            JsMutationKind::BinaryOp(Gt) => &["<", "<=", ">="],
            JsMutationKind::BinaryOp(Gte) => &[">", "<", "<="],
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
        let file = SourceFile::from_content(PathBuf::from("test.js"), content);
        (file, content.to_string())
    }

    #[test]
    fn finds_function_body() {
        let (f, content) = src("function foo() { return 1; }");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].span.start.byte, 15);
        assert_eq!(mutants[0].span.end.byte, 28);
        assert_eq!(mutants[0].kind, JsMutationKind::StatementBlock);
    }

    #[test]
    fn finds_nested_blocks() {
        let (f, content) = src("function foo() { if (x) { return 1; } }");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        assert_eq!(mutants.len(), 2);
    }

    #[test]
    fn kind_converts_to_unified() {
        let unified: MutationKind = JsMutationKind::StatementBlock.into();
        assert_eq!(unified, MutationKind::StatementBlock);
    }

    #[test]
    fn finds_addition() {
        let (f, content) = src("const x = a + b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::Add));
    }

    #[test]
    fn finds_multiplication() {
        let (f, content) = src("const x = a * b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::Mul));
    }

    #[test]
    fn finds_logical_and() {
        let (f, content) = src("const x = a && b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::And));
    }
}

use crate::{BinaryOpKind, Language, MutatedFile, MutationKind, SourceFile, Span};
use serde::{Deserialize, Serialize};

pub struct JavaScript;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
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

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn mutation_kind_for_node(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<(JsMutationKind, Span)> {
        match node.kind() {
            "statement_block" => {
                let span = Span { start: node.start_byte(), end: node.end_byte() };
                Some((JsMutationKind::StatementBlock, span))
            }
            "binary_expression" => {
                let op_node = node.child(1)?;
                let op = match op_node.utf8_text(source).ok()? {
                    "+"   => BinaryOpKind::Add,
                    "-"   => BinaryOpKind::Sub,
                    "*"   => BinaryOpKind::Mul,
                    "/"   => BinaryOpKind::Div,
                    "&&"  => BinaryOpKind::And,
                    "||"  => BinaryOpKind::Or,
                    "===" => BinaryOpKind::StrictEq,
                    "!==" => BinaryOpKind::StrictNeq,
                    "=="  => BinaryOpKind::Eq,
                    "!="  => BinaryOpKind::Neq,
                    "<"   => BinaryOpKind::Lt,
                    "<="  => BinaryOpKind::Lte,
                    ">"   => BinaryOpKind::Gt,
                    ">="  => BinaryOpKind::Gte,
                    _     => return None,
                };
                let span = Span { start: op_node.start_byte(), end: op_node.end_byte() };
                Some((JsMutationKind::BinaryOp(op), span))
            }
            _ => None,
        }
    }

    fn generate_substitutions<'a>(kind: &JsMutationKind, file: &'a SourceFile, span: &Span) -> Vec<MutatedFile<'a>> {
        use BinaryOpKind::*;
        let replacements: &[&str] = match kind {
            JsMutationKind::StatementBlock => &["{}"],
            JsMutationKind::BinaryOp(Add)       => &["-", "*", "/"],
            JsMutationKind::BinaryOp(Sub)       => &["+", "*", "/"],
            JsMutationKind::BinaryOp(Mul)       => &["+", "-", "/"],
            JsMutationKind::BinaryOp(Div)       => &["+", "-", "*"],
            JsMutationKind::BinaryOp(And)       => &["||"],
            JsMutationKind::BinaryOp(Or)        => &["&&"],
            JsMutationKind::BinaryOp(StrictEq)  => &["!=="],
            JsMutationKind::BinaryOp(StrictNeq) => &["==="],
            JsMutationKind::BinaryOp(Eq)        => &["!="],
            JsMutationKind::BinaryOp(Neq)       => &["=="],
            JsMutationKind::BinaryOp(Lt)        => &[">", "<=", ">="],
            JsMutationKind::BinaryOp(Lte)       => &["<", ">", ">="],
            JsMutationKind::BinaryOp(Gt)        => &["<", "<=", ">="],
            JsMutationKind::BinaryOp(Gte)       => &[">", "<", "<="],
        };
        replacements.iter().map(|r| file.with_replacement(span, r)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOpKind, SourceFile, Span, find_mutation_points};
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        let content = content.to_string();
        let hash = crate::Hash::of(&content);
        SourceFile { path: PathBuf::from("test.js"), content, hash }
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
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::Add));
    }

    #[test]
    fn finds_multiplication() {
        let f = file("const x = a * b;");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::Mul));
    }

    #[test]
    fn finds_logical_and() {
        let f = file("const x = a && b;");
        let points = find_mutation_points::<JavaScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, JsMutationKind::BinaryOp(BinaryOpKind::And));
    }
}

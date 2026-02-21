use crate::{BinaryOpKind, Language, MutatedFile, MutationKind, SourceFile, Span};
use serde::{Deserialize, Serialize};

pub struct TypeScript;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

    fn tree_sitter_language() -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn mutation_kind_for_node(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<(TsMutationKind, Span)> {
        match node.kind() {
            "statement_block" => {
                let span = Span { start: node.start_byte(), end: node.end_byte() };
                Some((TsMutationKind::StatementBlock, span))
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
                Some((TsMutationKind::BinaryOp(op), span))
            }
            _ => None,
        }
    }

    fn generate_substitutions<'a>(kind: &TsMutationKind, file: &'a SourceFile, span: &Span) -> Vec<(String, MutatedFile<'a>)> {
        use BinaryOpKind::*;
        let replacements: &[&str] = match kind {
            TsMutationKind::StatementBlock => &["{}"],
            TsMutationKind::BinaryOp(Add)       => &["-", "*", "/"],
            TsMutationKind::BinaryOp(Sub)       => &["+", "*", "/"],
            TsMutationKind::BinaryOp(Mul)       => &["+", "-", "/"],
            TsMutationKind::BinaryOp(Div)       => &["+", "-", "*"],
            TsMutationKind::BinaryOp(And)       => &["||"],
            TsMutationKind::BinaryOp(Or)        => &["&&"],
            TsMutationKind::BinaryOp(StrictEq)  => &["!=="],
            TsMutationKind::BinaryOp(StrictNeq) => &["==="],
            TsMutationKind::BinaryOp(Eq)        => &["!="],
            TsMutationKind::BinaryOp(Neq)       => &["=="],
            TsMutationKind::BinaryOp(Lt)        => &[">", "<=", ">="],
            TsMutationKind::BinaryOp(Lte)       => &["<", ">", ">="],
            TsMutationKind::BinaryOp(Gt)        => &["<", "<=", ">="],
            TsMutationKind::BinaryOp(Gte)       => &[">", "<", "<="],
        };
        replacements.iter().map(|r| (r.to_string(), file.with_replacement(span, r))).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOpKind, SourceFile, find_mutation_points};
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        let content = content.to_string();
        let hash = crate::Hash::of(&content);
        SourceFile { path: PathBuf::from("test.ts"), content, hash }
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
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::Add));
    }

    #[test]
    fn finds_multiplication() {
        let f = file("const x: number = a * b;");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::Mul));
    }

    #[test]
    fn finds_logical_and() {
        let f = file("const x: boolean = a && b;");
        let points = find_mutation_points::<TypeScript>(&f);
        assert_eq!(points.len(), 1);
        assert_eq!(points[0].kind, TsMutationKind::BinaryOp(BinaryOpKind::And));
    }
}

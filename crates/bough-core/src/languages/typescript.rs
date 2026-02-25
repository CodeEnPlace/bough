use crate::{BinaryOpKind, Language, MutationKind, SourceFile, Span};
use bough_sha::ShaHashable;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct TypeScript;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ShaHashable)]
pub enum TsMutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
    Condition,
}

impl From<TsMutationKind> for MutationKind {
    fn from(k: TsMutationKind) -> Self {
        match k {
            TsMutationKind::StatementBlock => MutationKind::StatementBlock,
            TsMutationKind::BinaryOp(op) => MutationKind::BinaryOp(op),
            TsMutationKind::Condition => MutationKind::Condition,
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
            "if_statement" | "while_statement" => {
                let cond = node.child_by_field_name("condition")?;
                let inner = cond.named_child(0)?;
                Some((TsMutationKind::Condition, Span::from_node(file, inner)))
            }
            "for_statement" => {
                let cond = node.child_by_field_name("condition")?;
                if cond.kind() == "empty_statement" { return None; }
                Some((TsMutationKind::Condition, Span::from_node(file, cond)))
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
            TsMutationKind::Condition => &["true", "false"],
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
    use crate::{BinaryOpKind, apply_mutation, find_mutants, generate_mutations};
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
        assert_eq!(mutants.len(), 3);
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

    #[test]
    fn condition_if_replaced_with_true_and_false() {
        let (f, content) = src("if (x > 0) { y = 1; }");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == TsMutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["if (true) { y = 1; }", "if (false) { y = 1; }"]);
    }

    #[test]
    fn condition_while_replaced_with_true_and_false() {
        let (f, content) = src("while (x > 0) { x--; }");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == TsMutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["while (true) { x--; }", "while (false) { x--; }"]);
    }

    #[test]
    fn condition_for_replaced_with_true_and_false() {
        let (f, content) = src("for (let i = 0; i < 10; i++) { x++; }");
        let mutants = find_mutants::<TypeScript>(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == TsMutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["for (let i = 0; true; i++) { x++; }", "for (let i = 0; false; i++) { x++; }"]);
    }
}

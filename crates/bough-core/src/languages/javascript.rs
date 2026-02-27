use crate::{BinaryOpKind, MutationKind, SourceFile, Span};
use super::LanguageDriver;

pub struct JavascriptDriver;

impl LanguageDriver for JavascriptDriver {
    fn tree_sitter_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn mutation_kind_for_node(
        &self,
        node: tree_sitter::Node<'_>,
        content: &[u8],
        file: &SourceFile,
    ) -> Option<(MutationKind, Span)> {
        match node.kind() {
            "statement_block" => {
                Some((MutationKind::StatementBlock, Span::from_node(file, node)))
            }
            "if_statement" | "while_statement" => {
                let cond = node.child_by_field_name("condition")?;
                let inner = cond.named_child(0)?;
                Some((MutationKind::Condition, Span::from_node(file, inner)))
            }
            "for_statement" => {
                let cond = node.child_by_field_name("condition")?;
                if cond.kind() == "empty_statement" { return None; }
                Some((MutationKind::Condition, Span::from_node(file, cond)))
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
                Some((MutationKind::BinaryOp(op), Span::from_node(file, op_node)))
            }
            _ => None,
        }
    }

    fn substitutions_for_kind(&self, kind: &MutationKind) -> Vec<String> {
        use BinaryOpKind::*;
        let replacements: &[&str] = match kind {
            MutationKind::StatementBlock => &["{}"],
            MutationKind::Condition => &["true", "false"],
            MutationKind::BinaryOp(Add) => &["-", "*", "/"],
            MutationKind::BinaryOp(Sub) => &["+", "*", "/"],
            MutationKind::BinaryOp(Mul) => &["+", "-", "/"],
            MutationKind::BinaryOp(Div) => &["+", "-", "*"],
            MutationKind::BinaryOp(And) => &["||"],
            MutationKind::BinaryOp(Or) => &["&&"],
            MutationKind::BinaryOp(StrictEq) => &["!=="],
            MutationKind::BinaryOp(StrictNeq) => &["==="],
            MutationKind::BinaryOp(Eq) => &["!="],
            MutationKind::BinaryOp(Neq) => &["=="],
            MutationKind::BinaryOp(Lt) => &[">", "<=", ">="],
            MutationKind::BinaryOp(Lte) => &["<", ">", ">="],
            MutationKind::BinaryOp(Gt) => &["<", "<=", ">="],
            MutationKind::BinaryOp(Gte) => &[">", "<", "<="],
        };
        replacements.iter().map(|r| r.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{BinaryOpKind, MutationKind, SourceFile, apply_mutation, find_mutants, generate_mutations};
    use crate::languages::LanguageId;
    use std::path::PathBuf;

    fn src(content: &str) -> (SourceFile, String) {
        let file = SourceFile::from_content(PathBuf::from("test.js"), content, LanguageId::Javascript);
        (file, content.to_string())
    }

    #[test]
    fn finds_function_body() {
        let (f, content) = src("function foo() { return 1; }");
        let mutants = find_mutants(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].span.start.byte, 15);
        assert_eq!(mutants[0].span.end.byte, 28);
        assert_eq!(mutants[0].kind, MutationKind::StatementBlock);
    }

    #[test]
    fn finds_nested_blocks() {
        let (f, content) = src("function foo() { if (x) { return 1; } }");
        let mutants = find_mutants(&f, &content);
        assert_eq!(mutants.len(), 3);
    }

    #[test]
    fn finds_addition() {
        let (f, content) = src("const x = a + b;");
        let mutants = find_mutants(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, MutationKind::BinaryOp(BinaryOpKind::Add));
    }

    #[test]
    fn finds_multiplication() {
        let (f, content) = src("const x = a * b;");
        let mutants = find_mutants(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, MutationKind::BinaryOp(BinaryOpKind::Mul));
    }

    #[test]
    fn finds_logical_and() {
        let (f, content) = src("const x = a && b;");
        let mutants = find_mutants(&f, &content);
        assert_eq!(mutants.len(), 1);
        assert_eq!(mutants[0].kind, MutationKind::BinaryOp(BinaryOpKind::And));
    }

    #[test]
    fn condition_if_replaced_with_true_and_false() {
        let (f, content) = src("if (x > 0) { y = 1; }");
        let mutants = find_mutants(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == MutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["if (true) { y = 1; }", "if (false) { y = 1; }"]);
    }

    #[test]
    fn condition_while_replaced_with_true_and_false() {
        let (f, content) = src("while (x > 0) { x--; }");
        let mutants = find_mutants(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == MutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["while (true) { x--; }", "while (false) { x--; }"]);
    }

    #[test]
    fn condition_for_replaced_with_true_and_false() {
        let (f, content) = src("for (let i = 0; i < 10; i++) { x++; }");
        let mutants = find_mutants(&f, &content);
        let cond = mutants.iter().find(|m| m.kind == MutationKind::Condition).unwrap();
        let mutations = generate_mutations(cond);
        let applied: Vec<_> = mutations.iter().map(|m| apply_mutation(&content, &m.mutant.span, &m.replacement)).collect();
        assert_eq!(applied, vec!["for (let i = 0; true; i++) { x++; }", "for (let i = 0; false; i++) { x++; }"]);
    }
}

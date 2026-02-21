use crate::{BinaryOpKind, Language, MutationKind};

pub struct JavaScript;

#[derive(Debug, PartialEq)]
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

    fn mutation_kind_for_node(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<JsMutationKind> {
        match node.kind() {
            "statement_block" => Some(JsMutationKind::StatementBlock),
            "binary_expression" => {
                let op_str = node.child(1)?.utf8_text(source).ok()?;
                Some(JsMutationKind::BinaryOp(BinaryOpKind::from_str(op_str)?))
            }
            _ => None,
        }
    }

    fn generate_substitutions(kind: &JsMutationKind, span_text: &str) -> Vec<String> {
        match kind {
            JsMutationKind::StatementBlock => vec!["{}".to_string()],
            JsMutationKind::BinaryOp(op) => {
                let op_str = op.as_str();
                let pos = match span_text.find(op_str) {
                    Some(p) => p,
                    None => return vec![],
                };
                let lhs = &span_text[..pos];
                let rhs = &span_text[pos + op_str.len()..];
                op.alternatives().iter().map(|alt| format!("{}{}{}", lhs, alt.as_str(), rhs)).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BinaryOpKind, SourceFile, Span, find_mutation_points};
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

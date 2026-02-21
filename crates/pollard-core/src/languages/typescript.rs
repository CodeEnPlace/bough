use crate::{BinaryOpKind, Language, MutationKind};

pub struct TypeScript;

#[derive(Debug, PartialEq)]
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

    fn mutation_kind_for_node(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<TsMutationKind> {
        match node.kind() {
            "statement_block" => Some(TsMutationKind::StatementBlock),
            "binary_expression" => {
                let op_str = node.child(1)?.utf8_text(source).ok()?;
                Some(TsMutationKind::BinaryOp(BinaryOpKind::from_str(op_str)?))
            }
            _ => None,
        }
    }

    fn generate_substitutions(kind: &TsMutationKind, span_text: &str) -> Vec<String> {
        match kind {
            TsMutationKind::StatementBlock => vec!["{}".to_string()],
            TsMutationKind::BinaryOp(op) => {
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
    use crate::{BinaryOpKind, SourceFile, find_mutation_points};
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

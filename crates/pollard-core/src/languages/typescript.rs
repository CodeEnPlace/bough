use crate::{Language, MutationKind};

pub struct TypeScript;

#[derive(Debug, PartialEq)]
pub enum TsMutationKind {
    StatementBlock,
}

impl From<TsMutationKind> for MutationKind {
    fn from(k: TsMutationKind) -> Self {
        match k {
            TsMutationKind::StatementBlock => MutationKind::StatementBlock,
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
            _ => None,
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
}

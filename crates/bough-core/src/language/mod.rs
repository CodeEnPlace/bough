use crate::mutant::{MutantKind, Span};

pub trait LanguageDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language;

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)>;

    fn substitutions(&self, kind: &MutantKind) -> Vec<String>;

    fn is_context_boundary(&self, node: &arborium_tree_sitter::Node<'_>) -> bool;
}

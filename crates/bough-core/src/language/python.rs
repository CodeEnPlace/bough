use super::LanguageDriver;
use crate::mutant::{MutantKind, Span};

pub(crate) struct PythonDriver;

impl LanguageDriver for PythonDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_python::language().into()
    }

    fn check_node(
        &self,
        _node: &arborium_tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        None
    }

    fn substitutions(&self, _kind: &MutantKind) -> Vec<String> {
        vec![]
    }

    fn is_context_boundary(&self, _node: &arborium_tree_sitter::Node<'_>) -> bool {
        false
    }
}

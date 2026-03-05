use super::LanguageDriver;
use crate::mutant::{MutantKind, Span};

pub(crate) struct TypescriptDriver;

impl LanguageDriver for TypescriptDriver {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn check_node(
        &self,
        _node: &tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        None
    }

    // core[impl mutation.iter.invalid]
    fn substitutions(&self, _kind: &MutantKind) -> Vec<String> {
        vec![]
    }
}

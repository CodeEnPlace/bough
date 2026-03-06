mod javascript;
mod typescript;

pub(crate) use javascript::JavascriptDriver;
pub(crate) use typescript::TypescriptDriver;

use crate::mutant::{MutantKind, Span};
use tracing::debug;

// core[impl mutation.iter.language_driver]
pub(crate) trait LanguageDriver {
    fn ts_language(&self) -> tree_sitter::Language;

    fn check_node(
        &self,
        node: &tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)>;

    fn substitutions(&self, kind: &MutantKind) -> Vec<String>;
}

pub(crate) fn driver_for_lang(lang: crate::LanguageId) -> Box<dyn LanguageDriver> {
    debug!(?lang, "selecting language driver");
    match lang {
        crate::LanguageId::Javascript => Box::new(JavascriptDriver),
        crate::LanguageId::Typescript => Box::new(TypescriptDriver),
    }
}

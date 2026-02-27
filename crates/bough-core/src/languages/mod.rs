pub mod javascript;
pub mod typescript;

pub use javascript::JavascriptDriver;
pub use typescript::TypescriptDriver;

use crate::{MutationKind, SourceFile, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize, clap::ValueEnum, bough_typed_hash::HashInto)]
#[serde(rename_all = "lowercase")]
pub enum LanguageId {
    #[serde(alias = "js")]
    #[value(alias = "js")]
    Javascript,
    #[serde(alias = "ts")]
    #[value(alias = "ts")]
    Typescript,
}

pub trait LanguageDriver {
    fn tree_sitter_language(&self) -> tree_sitter::Language;
    fn mutation_kind_for_node(
        &self,
        node: tree_sitter::Node<'_>,
        content: &[u8],
        file: &SourceFile,
    ) -> Option<(MutationKind, Span)>;
    fn substitutions_for_kind(&self, kind: &MutationKind) -> Vec<String>;
}

pub fn driver_for(lang: LanguageId) -> &'static dyn LanguageDriver {
    match lang {
        LanguageId::Javascript => &JavascriptDriver,
        LanguageId::Typescript => &TypescriptDriver,
    }
}

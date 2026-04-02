mod c;
mod cpp;
mod cs;
mod go;
mod java;
mod javascript;
mod python;
mod rb;
mod rs;
mod swift;
mod typescript;
mod zig;

pub(crate) use c::CDriver;
pub(crate) use cpp::CppDriver;
pub(crate) use cs::CSharpDriver;
pub(crate) use go::GoDriver;
pub(crate) use java::JavaDriver;
pub(crate) use javascript::JavascriptDriver;
pub(crate) use python::PythonDriver;
pub(crate) use rb::RubyDriver;
pub(crate) use rs::RustDriver;
pub(crate) use swift::SwiftDriver;
pub(crate) use typescript::TypescriptDriver;
pub(crate) use zig::ZigDriver;

use crate::mutant::{MutantKind, Span};

// bough[impl mutation.iter.language_driver]
pub(crate) trait LanguageDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language;

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)>;

    fn substitutions(&self, kind: &MutantKind) -> Vec<String>;

    fn is_context_boundary(&self, node: &arborium_tree_sitter::Node<'_>) -> bool;
}

pub(crate) fn driver_for_lang(lang: crate::LanguageId) -> Box<dyn LanguageDriver> {
    match lang {
        crate::LanguageId::Javascript => Box::new(JavascriptDriver),
        crate::LanguageId::Typescript => Box::new(TypescriptDriver),
        crate::LanguageId::Python => Box::new(PythonDriver),
        crate::LanguageId::C => Box::new(CDriver),
        crate::LanguageId::Go => Box::new(GoDriver),
        crate::LanguageId::Java => Box::new(JavaDriver),
        crate::LanguageId::CSharp => Box::new(CSharpDriver),
        crate::LanguageId::Rust => Box::new(RustDriver),
        crate::LanguageId::Swift => Box::new(SwiftDriver),
        crate::LanguageId::Ruby => Box::new(RubyDriver),
        crate::LanguageId::Zig => Box::new(ZigDriver),
        crate::LanguageId::Cpp => Box::new(CppDriver),
    }
}

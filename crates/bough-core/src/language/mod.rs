pub mod c;
pub mod cpp;
pub mod cs;
pub mod go;
pub mod java;
pub mod javascript;
pub mod python;
pub mod rb;
pub mod rs;
pub mod swift;
pub mod typescript;
pub mod zig;

pub use c::CDriver;
pub use cpp::CppDriver;
pub use cs::CSharpDriver;
pub use go::GoDriver;
pub use java::JavaDriver;
pub use javascript::JavascriptDriver;
pub use python::PythonDriver;
pub use rb::RubyDriver;
pub use rs::RustDriver;
pub use swift::SwiftDriver;
pub use typescript::TypescriptDriver;
pub use zig::ZigDriver;

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

pub fn driver_for_lang(lang: crate::LanguageId) -> Box<dyn LanguageDriver> {
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

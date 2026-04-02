pub use bough_core::language::LanguageDriver;

pub fn driver_for_lang(lang: bough_core::LanguageId) -> Box<dyn LanguageDriver> {
    match lang {
        bough_core::LanguageId::Javascript => Box::new(bough_lang_javascript::JavascriptDriver),
        bough_core::LanguageId::Typescript => Box::new(bough_lang_typescript::TypescriptDriver),
        bough_core::LanguageId::Python => Box::new(bough_lang_python::PythonDriver),
        bough_core::LanguageId::C => Box::new(bough_lang_c::CDriver),
        bough_core::LanguageId::Go => Box::new(bough_lang_go::GoDriver),
        bough_core::LanguageId::Java => Box::new(bough_lang_java::JavaDriver),
        bough_core::LanguageId::CSharp => Box::new(bough_lang_cs::CSharpDriver),
        bough_core::LanguageId::Rust => Box::new(bough_lang_rust::RustDriver),
        bough_core::LanguageId::Swift => Box::new(bough_lang_swift::SwiftDriver),
        bough_core::LanguageId::Ruby => Box::new(bough_lang_ruby::RubyDriver),
        bough_core::LanguageId::Zig => Box::new(bough_lang_zig::ZigDriver),
        bough_core::LanguageId::Cpp => Box::new(bough_lang_cpp::CppDriver),
    }
}

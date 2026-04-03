#![allow(dead_code)]

pub mod language;
pub mod mutant;
mod mutation;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    facet::Facet,
    bough_typed_hash::HashInto,
)]
#[facet(rename_all = "lowercase")]
#[repr(u8)]
pub enum LanguageId {
    #[facet(rename = "js")]
    Javascript,
    #[facet(rename = "ts")]
    Typescript,
    #[facet(rename = "py")]
    Python,
    #[facet(rename = "c")]
    C,
    #[facet(rename = "go")]
    Go,
    #[facet(rename = "java")]
    Java,
    #[facet(rename = "cs")]
    CSharp,
    #[facet(rename = "rs")]
    Rust,
    #[facet(rename = "swift")]
    Swift,
    #[facet(rename = "rb")]
    Ruby,
    #[facet(rename = "zig")]
    Zig,
    #[facet(rename = "cpp")]
    Cpp,
}

impl std::fmt::Display for LanguageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.slug())
    }
}

impl LanguageId {
    pub const ALL: &[LanguageId] = &[
        LanguageId::Javascript,
        LanguageId::Typescript,
        LanguageId::Python,
        LanguageId::C,
        LanguageId::Go,
        LanguageId::Java,
        LanguageId::CSharp,
        LanguageId::Rust,
        LanguageId::Swift,
        LanguageId::Ruby,
        LanguageId::Zig,
        LanguageId::Cpp,
    ];

    pub fn slug(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "js",
            LanguageId::Typescript => "ts",
            LanguageId::Python => "py",
            LanguageId::C => "c",
            LanguageId::Go => "go",
            LanguageId::Java => "java",
            LanguageId::CSharp => "cs",
            LanguageId::Rust => "rs",
            LanguageId::Swift => "swift",
            LanguageId::Ruby => "rb",
            LanguageId::Zig => "zig",
            LanguageId::Cpp => "cpp",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "JavaScript",
            LanguageId::Typescript => "TypeScript",
            LanguageId::Python => "Python",
            LanguageId::C => "C",
            LanguageId::Go => "Go",
            LanguageId::Java => "Java",
            LanguageId::CSharp => "C#",
            LanguageId::Rust => "Rust",
            LanguageId::Swift => "Swift",
            LanguageId::Ruby => "Ruby",
            LanguageId::Zig => "Zig",
            LanguageId::Cpp => "C++",
        }
    }

    pub fn corpus_dir_name(&self) -> &'static str {
        match self {
            LanguageId::Javascript => "javascript",
            LanguageId::Typescript => "typescript",
            LanguageId::Python => "python",
            LanguageId::C => "c",
            LanguageId::Go => "go",
            LanguageId::Java => "java",
            LanguageId::CSharp => "c-sharp",
            LanguageId::Rust => "rust",
            LanguageId::Swift => "swift",
            LanguageId::Ruby => "ruby",
            LanguageId::Zig => "zig",
            LanguageId::Cpp => "cpp",
        }
    }

    pub fn file_extension(&self) -> &'static str {
        self.slug()
    }
}

pub use bough_fs::File;
pub use bough_fs::Root;
pub use bough_fs::Twig;
pub use bough_glob::TwigsIterBuilder;
pub use mutant::{Mutant, MutantKind, Point, SourceMutant, Span, find_mutants_in_source};
pub use mutation::Mutation;
pub use mutation::MutationHash;
pub use mutation::MutationIter;

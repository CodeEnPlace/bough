pub mod apply;
pub mod describe;
pub mod generate;
pub mod view;

use bough_core::config::LanguageId;
use std::path::PathBuf as StdPathBuf;

pub fn expand_glob(pattern: &str) -> Vec<StdPathBuf> {
    glob::glob(pattern)
        .unwrap_or_else(|e| {
            eprintln!("invalid glob pattern: {e}");
            std::process::exit(1);
        })
        .filter_map(|entry| match entry {
            Ok(path) if path.is_file() => Some(path),
            Ok(_) => None,
            Err(e) => {
                eprintln!("glob error: {e}");
                None
            }
        })
        .collect()
}
use bough_core::languages::javascript::JavaScript;
use bough_core::languages::typescript::TypeScript;
use bough_core::{
    Hash, Language, MutatedFile, MutationKind, SourceFile, find_mutation_points,
    generate_mutation_substitutions,
};
use std::path::PathBuf;

fn find_mutated_by_hash<'a, L: Language>(
    file: &'a SourceFile,
    target: &Hash,
) -> Option<MutatedFile<'a>> {
    let points = find_mutation_points::<L>(file);
    for point in &points {
        for (_, mutated) in generate_mutation_substitutions::<L>(point) {
            if mutated.hash() == target {
                return Some(mutated);
            }
        }
    }
    None
}

fn find_mutated<'a>(
    language: &LanguageId,
    file: &'a SourceFile,
    hash: &Hash,
) -> MutatedFile<'a> {
    match language {
        LanguageId::Javascript => find_mutated_by_hash::<JavaScript>(file, hash),
        LanguageId::Typescript => find_mutated_by_hash::<TypeScript>(file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    })
}

pub struct MutationDescription {
    pub path: PathBuf,
    pub kind: MutationKind,
    pub code_tag: &'static str,
    pub start_line: usize,
    pub start_char: usize,
    pub end_line: usize,
    pub end_char: usize,
    pub original: String,
    pub replacement: String,
    pub mutated_hash: Hash,
}

fn find_description_by_hash<L: Language>(
    file: &SourceFile,
    target: &Hash,
) -> Option<MutationDescription>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(&point) {
            if mutated.hash() == target {
                return Some(MutationDescription {
                    path: file.path().to_owned(),
                    kind: point.kind.into(),
                    code_tag: L::code_tag(),
                    start_line: point.span.start.line,
                    start_char: point.span.start.char,
                    end_line: point.span.end.line,
                    end_char: point.span.end.char,
                    original: file.content()[point.span.start.byte..point.span.end.byte]
                        .to_string(),
                    replacement,
                    mutated_hash: mutated.hash().clone(),
                });
            }
        }
    }
    None
}

fn find_description(
    language: &LanguageId,
    file: &SourceFile,
    hash: &Hash,
) -> MutationDescription {
    match language {
        LanguageId::Javascript => find_description_by_hash::<JavaScript>(file, hash),
        LanguageId::Typescript => find_description_by_hash::<TypeScript>(file, hash),
    }
    .unwrap_or_else(|| {
        eprintln!("no mutation found with hash {hash}");
        std::process::exit(1);
    })
}

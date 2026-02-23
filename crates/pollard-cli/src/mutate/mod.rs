pub mod apply;
pub mod generate;
pub mod view;

use pollard_core::config::LanguageId;
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::{
    Hash, Language, MutatedFile, SourceFile, find_mutation_points, generate_mutation_substitutions,
};
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

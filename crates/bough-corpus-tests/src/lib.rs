#[cfg(test)]
use bough_core::mutant::TwigMutantsIter;
#[cfg(test)]
use bough_core::{Base, LanguageId, Mutation, MutationIter, Twig, TwigsIterBuilder};
#[cfg(test)]
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::collections::HashSet;
#[cfg(test)]
use std::path::{Path, PathBuf};

#[cfg(test)]
fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..6])
}

#[cfg(test)]
fn corpus_test_runner(case_dir: &str, lang: LanguageId, ext: &str) {
    let case_path = Path::new(case_dir);
    let base_file = case_path.join(format!("base.{ext}"));
    let src = std::fs::read_to_string(&base_file)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", base_file.display()));

    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("src")).unwrap();
    std::fs::write(dir.path().join(format!("src/a.{ext}")), &src).unwrap();

    let base = Base::new(
        dir.path().to_path_buf(),
        TwigsIterBuilder::new().with_include_glob(&format!("src/**/*.{ext}")),
    )
    .unwrap();

    let twig = Twig::new(PathBuf::from(format!("src/a.{ext}"))).unwrap();

    let mutations: Vec<(String, Mutation)> = TwigMutantsIter::new(lang, &base, &twig)
        .unwrap()
        .flat_map(|bm| {
            let mutant = bm.into_mutant();
            MutationIter::new(&mutant)
                .map(|mutation| {
                    let mutated_src = mutation.apply_to_complete_src_string(&src);
                    (mutated_src, mutation)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let mut produced_files: HashSet<String> = HashSet::new();
    let update_mode = std::env::var("BOUGH_UPDATE_CORPUS").is_ok();

    for (mutated_src, mutation) in &mutations {
        let hash = content_hash(mutated_src);
        let src_file = format!("{hash}.{ext}");
        let json_file = format!("{hash}.mutation.json");
        produced_files.insert(src_file.clone());
        produced_files.insert(json_file.clone());

        let src_path = case_path.join(&src_file);
        let json_path = case_path.join(&json_file);

        if !src_path.exists() {
            std::fs::write(&src_path, mutated_src)
                .unwrap_or_else(|e| panic!("failed to write {}: {e}", src_path.display()));
            eprintln!("wrote new mutation: {}", src_path.display());
        } else {
            let existing = std::fs::read_to_string(&src_path).unwrap();
            assert_eq!(
                existing,
                *mutated_src,
                "mutation content mismatch in {}",
                src_path.display()
            );
        }

        let json = facet_json::to_string_pretty(mutation)
            .unwrap_or_else(|e| panic!("failed to serialize mutation to json: {e}"));
        if !json_path.exists() {
            std::fs::write(&json_path, &json)
                .unwrap_or_else(|e| panic!("failed to write {}: {e}", json_path.display()));
            eprintln!("wrote new mutation json: {}", json_path.display());
        } else {
            let existing = std::fs::read_to_string(&json_path).unwrap();
            assert_eq!(
                existing,
                json,
                "mutation json mismatch in {}",
                json_path.display()
            );
        }
    }

    produced_files.insert(format!("base.{ext}"));

    let existing_files: HashSet<String> = std::fs::read_dir(case_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|e| e.file_name().to_str().unwrap().to_string())
        .collect();

    let stale: Vec<_> = existing_files.difference(&produced_files).collect();

    if !stale.is_empty() {
        if update_mode {
            for file in &stale {
                let path = case_path.join(file);
                std::fs::remove_file(&path)
                    .unwrap_or_else(|e| panic!("failed to remove stale {}: {e}", path.display()));
                eprintln!("removed stale file: {}", path.display());
            }
        } else {
            let mut stale_sorted: Vec<_> = stale.into_iter().collect();
            stale_sorted.sort();
            panic!(
                "stale files in {}: {:?}\nSet BOUGH_UPDATE_CORPUS=1 to auto-remove",
                case_path.display(),
                stale_sorted
            );
        }
    }
}

#[cfg(test)]
#[allow(non_snake_case)]
mod tests {
    include!(concat!(env!("OUT_DIR"), "/corpus_tests.rs"));
}

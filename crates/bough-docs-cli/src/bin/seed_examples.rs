use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const CORPUS_DIR: &str = "corpus";
const EXAMPLES_DIR: &str = "docs/mutations";

fn main() {
    for lang in bough_core::LanguageId::ALL {
        seed_language(lang);
    }
}

fn seed_language(lang: &bough_core::LanguageId) {
    let corpus_dir = Path::new(CORPUS_DIR).join(lang.corpus_dir_name());
    let ext = lang.file_extension();

    if !corpus_dir.exists() {
        eprintln!("  skipping {} (no corpus dir)", lang.display_name());
        return;
    }

    // For each kind, track the shortest snippet that contains it
    let mut best: BTreeMap<String, (String, usize)> = BTreeMap::new(); // key -> (snippet, len)

    let mut case_dirs: Vec<_> = fs::read_dir(&corpus_dir)
        .expect("failed to read corpus dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    case_dirs.sort_by_key(|e| e.file_name());

    for case_entry in case_dirs {
        let case_path = case_entry.path();
        let base_path = case_path.join(format!("base.{ext}"));
        let Ok(source) = fs::read_to_string(&base_path) else {
            continue;
        };
        if source.is_empty() {
            continue;
        }

        let mutants = bough_core::find_mutants_in_source(bough_core::language::driver_for_lang(*lang).as_ref(), source.as_bytes());
        for mutant in &mutants {
            let key = mutant.kind.to_key();
            let subs = bough_core::language::driver_for_lang(*lang).substitutions(&mutant.kind);
            if subs.is_empty() {
                continue;
            }

            let source_len = source.len();
            if best
                .get(&key)
                .is_none_or(|(_, prev_len)| source_len < *prev_len)
            {
                best.insert(key, (source.clone(), source_len));
            }
        }
    }

    // Write the TOML file
    let out_path = Path::new(EXAMPLES_DIR).join(format!("{}.examples.toml", lang.slug()));

    let mut out = String::new();
    for kind in bough_core::MutantKind::all_variants() {
        let subs = bough_core::language::driver_for_lang(*lang).substitutions(&kind);
        if subs.is_empty() {
            continue;
        }

        let key = kind.to_key();
        if let Some((snippet, _)) = best.get(&key) {
            out.push_str("[[mutation]]\n");
            out.push_str(&format!("kind = \"{key}\"\n"));
            out.push_str(&format!(
                "snippet = \"\"\"\n{}\"\"\"\n\n",
                snippet.trim_end()
            ));
        } else {
            eprintln!(
                "  WARNING: {} - no corpus example for kind '{}'",
                lang.display_name(),
                key
            );
            out.push_str("[[mutation]]\n");
            out.push_str(&format!("kind = \"{key}\"\n"));
            out.push_str("snippet = \"\"\"TODO\"\"\"\n\n");
        }
    }

    fs::write(&out_path, &out).unwrap_or_else(|e| {
        panic!("failed to write {}: {e}", out_path.display());
    });
    eprintln!("  wrote {} ({} examples)", out_path.display(), best.len());
}

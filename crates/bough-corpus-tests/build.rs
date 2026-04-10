use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    let corpus_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../corpus");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let gen_path = Path::new(&out_dir).join("corpus_tests.rs");

    println!("cargo:rerun-if-changed=../../corpus");

    let mut out = fs::File::create(&gen_path).unwrap();

    if !corpus_dir.exists() {
        return;
    }

    let mut lang_dirs: Vec<_> = fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    lang_dirs.sort_by_key(|e| e.file_name());

    let mut modules: BTreeMap<String, Vec<(String, String, String, String)>> = BTreeMap::new();

    for lang_entry in lang_dirs {
        let lang_name = lang_entry.file_name();
        let lang_str = lang_name.to_str().unwrap();

        let lang_id = match lang_str {
            "javascript" => "bough_core::LanguageId::Javascript",
            "typescript" => "bough_core::LanguageId::Typescript",
            "python" => "bough_core::LanguageId::Python",
            "c" => "bough_core::LanguageId::C",
            "go" => "bough_core::LanguageId::Go",
            "java" => "bough_core::LanguageId::Java",
            "c-sharp" => "bough_core::LanguageId::CSharp",
            "rust" => "bough_core::LanguageId::Rust",
            "swift" => "bough_core::LanguageId::Swift",
            "ruby" => "bough_core::LanguageId::Ruby",
            "zig" => "bough_core::LanguageId::Zig",
            "cpp" => "bough_core::LanguageId::Cpp",
            _ => continue,
        };

        let ext = match lang_str {
            "javascript" => "js",
            "typescript" => "ts",
            "python" => "py",
            "c" => "c",
            "go" => "go",
            "java" => "java",
            "c-sharp" => "cs",
            "rust" => "rs",
            "swift" => "swift",
            "ruby" => "rb",
            "zig" => "zig",
            "cpp" => "cpp",
            _ => continue,
        };

        let mut case_dirs: Vec<_> = fs::read_dir(lang_entry.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .collect();
        case_dirs.sort_by_key(|e| e.file_name());

        let cases: Vec<_> = case_dirs
            .into_iter()
            .filter_map(|case_entry| {
                let case_name = case_entry.file_name();
                let case_str = case_name.to_str().unwrap().to_string();
                let base_file = case_entry.path().join(format!("base.{ext}"));
                if !base_file.exists() {
                    return None;
                }
                let fn_name = case_str.replace('-', "_");
                let case_path = case_entry
                    .path()
                    .canonicalize()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .replace('\\', "/");
                Some((fn_name, case_path, lang_id.to_string(), ext.to_string()))
            })
            .collect();

        modules.insert(lang_str.to_string(), cases);
    }

    for (lang, cases) in &modules {
        let mod_name = lang.replace('-', "_");
        writeln!(out, "mod {mod_name} {{").unwrap();
        for (fn_name, case_path, lang_id, ext) in cases {
            writeln!(out, "    #[test]").unwrap();
            writeln!(out, "    fn {fn_name}() {{").unwrap();
            writeln!(
                out,
                "        crate::corpus_test_runner(\"{case_path}\", {lang_id}, \"{ext}\");",
            )
            .unwrap();
            writeln!(out, "    }}").unwrap();
            writeln!(out).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }
}

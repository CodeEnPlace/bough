use arbitrary::Arbitrary;
use bough_fs::{TestRoot, Twig};
use bough_glob::{Glob, TwigWalker};
use std::collections::BTreeSet;

const SEGMENTS: &[&str] = &[
    "src",
    "lib",
    "test",
    "build",
    "dist",
    "vendor",
    "node_modules",
    "docs",
    ".hidden",
    "a",
    "b",
    "c",
];
const EXTENSIONS: &[&str] = &["js", "ts", "py", "rs", "css", "md", "json", "txt"];
const GLOB_WILDCARDS: &[&str] = &["*", "**", "**/*"];

fn arb_segment(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<&'static str> {
    Ok(*u.choose(SEGMENTS)?)
}

fn arb_filename(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<String> {
    let name = arb_segment(u)?;
    let ext = *u.choose(EXTENSIONS)?;
    Ok(format!("{name}.{ext}"))
}

fn arb_file_path(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<String> {
    let depth = u.int_in_range(1..=3)?;
    let parts: Vec<&str> = (0..depth)
        .map(|_| arb_segment(u))
        .collect::<arbitrary::Result<_>>()?;
    let dir = parts.join("/");
    let file = arb_filename(u)?;
    Ok(format!("{dir}/{file}"))
}

fn arb_glob_pattern(u: &mut arbitrary::Unstructured<'_>) -> arbitrary::Result<String> {
    let style: u8 = u.int_in_range(0..=6)?;
    match style {
        0 => {
            let ext = *u.choose(EXTENSIONS)?;
            Ok(format!("**/*.{ext}"))
        }
        1 => {
            let seg = arb_segment(u)?;
            let ext = *u.choose(EXTENSIONS)?;
            Ok(format!("{seg}/**/*.{ext}"))
        }
        2 => {
            let seg = arb_segment(u)?;
            let wc = *u.choose(GLOB_WILDCARDS)?;
            Ok(format!("{seg}/{wc}"))
        }
        3 => {
            let depth = u.int_in_range(1..=2)?;
            let parts: Vec<&str> = (0..depth)
                .map(|_| arb_segment(u))
                .collect::<arbitrary::Result<_>>()?;
            let path = parts.join("/");
            let wc = *u.choose(GLOB_WILDCARDS)?;
            Ok(format!("{path}/{wc}"))
        }
        4 => arb_file_path(u),
        5 => {
            let ext1 = *u.choose(EXTENSIONS)?;
            let ext2 = *u.choose(EXTENSIONS)?;
            Ok(format!("**/*.{{{ext1},{ext2}}}"))
        }
        6 => {
            let seg = arb_segment(u)?;
            let ext = *u.choose(EXTENSIONS)?;
            Ok(format!("{seg}/**/[a-z]*.{ext}"))
        }
        _ => unreachable!(),
    }
}

#[derive(Debug)]
struct TestCase {
    files: Vec<String>,
    includes: Vec<String>,
    excludes: Vec<String>,
}

impl<'a> Arbitrary<'a> for TestCase {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        let n_files = u.int_in_range(1..=20)?;
        let files: Vec<String> = (0..n_files)
            .map(|_| arb_file_path(u))
            .collect::<arbitrary::Result<_>>()?;

        let n_includes = u.int_in_range(1..=3)?;
        let includes: Vec<String> = (0..n_includes)
            .map(|_| arb_glob_pattern(u))
            .collect::<arbitrary::Result<_>>()?;

        let n_excludes = u.int_in_range(0..=2)?;
        let excludes: Vec<String> = (0..n_excludes)
            .map(|_| arb_glob_pattern(u))
            .collect::<arbitrary::Result<_>>()?;

        Ok(TestCase {
            files,
            includes,
            excludes,
        })
    }
}

fn reference_walk(
    root: &std::path::Path,
    includes: &[globset::GlobMatcher],
    excludes: &[globset::GlobMatcher],
) -> BTreeSet<Twig> {
    let mut result = BTreeSet::new();
    walk_dir_recursive(root, root, includes, excludes, &mut result);
    result
}

fn walk_dir_recursive(
    root: &std::path::Path,
    dir: &std::path::Path,
    includes: &[globset::GlobMatcher],
    excludes: &[globset::GlobMatcher],
    result: &mut BTreeSet<Twig>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap();

        if path.is_file() {
            let matched_include = includes.iter().any(|g| g.is_match(rel));
            let matched_exclude = excludes.iter().any(|g| g.is_match(rel));
            if matched_include && !matched_exclude {
                if let Ok(twig) = Twig::new(rel.to_path_buf()) {
                    result.insert(twig);
                }
            }
        } else if path.is_dir() {
            walk_dir_recursive(root, &path, includes, excludes, result);
        }
    }
}

#[test]
fn pbt_twig_walker_matches_reference() {
    arbtest::arbtest(|u| {
        let tc = TestCase::arbitrary(u)?;

        let dir = tempfile::tempdir().unwrap();
        let root_path = dir.path();

        for f in &tc.files {
            let p = root_path.join(f);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, "x").unwrap();
        }

        let our_includes: Vec<Glob> = tc
            .includes
            .iter()
            .map(|s| Glob::try_from(s.as_str()).unwrap())
            .collect();
        let our_excludes: Vec<Glob> = tc
            .excludes
            .iter()
            .map(|s| Glob::try_from(s.as_str()).unwrap())
            .collect();

        let ref_includes: Vec<globset::GlobMatcher> = tc
            .includes
            .iter()
            .map(|s| {
                globset::GlobBuilder::new(s)
                    .literal_separator(true)
                    .build()
                    .unwrap()
                    .compile_matcher()
            })
            .collect();
        let ref_excludes: Vec<globset::GlobMatcher> = tc
            .excludes
            .iter()
            .map(|s| {
                globset::GlobBuilder::new(s)
                    .literal_separator(true)
                    .build()
                    .unwrap()
                    .compile_matcher()
            })
            .collect();

        let root = TestRoot::new(root_path);
        let mut walker = TwigWalker::new(&root);
        for g in our_includes {
            walker = walker.include(g);
        }
        for g in our_excludes {
            walker = walker.exclude(g);
        }
        let our_result: BTreeSet<Twig> = walker.iter().collect();
        let ref_result = reference_walk(root_path, &ref_includes, &ref_excludes);

        assert_eq!(
            our_result, ref_result,
            "mismatch for includes={:?} excludes={:?} files={:?}",
            tc.includes, tc.excludes, tc.files
        );

        Ok(())
    })
    .budget(std::time::Duration::from_secs(1));
}

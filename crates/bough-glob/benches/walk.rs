use bough_fs::{TestRoot, Twig};
use bough_glob::{Glob, TwigWalker};
use criterion::{Criterion, criterion_group, criterion_main};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn make_deep_tree(root: &Path) {
    for i in 0..10 {
        for j in 0..5 {
            let dir = root.join(format!("d{i}/sub{j}/deep/nested/more"));
            std::fs::create_dir_all(&dir).unwrap();
            for k in 0..10 {
                std::fs::write(dir.join(format!("file{k}.js")), "x").unwrap();
                std::fs::write(dir.join(format!("file{k}.ts")), "x").unwrap();
                std::fs::write(dir.join(format!("file{k}.css")), "x").unwrap();
            }
        }
    }
}

fn make_wide_tree(root: &Path) {
    for i in 0..100 {
        let dir = root.join(format!("pkg{i}/src"));
        std::fs::create_dir_all(&dir).unwrap();
        for j in 0..20 {
            std::fs::write(dir.join(format!("mod{j}.js")), "x").unwrap();
            std::fs::write(dir.join(format!("mod{j}.css")), "x").unwrap();
        }
    }
}

fn make_mixed_tree(root: &Path) {
    for i in 0..20 {
        for j in 0..10 {
            let dir = root.join(format!("area{i}/lib{j}/src"));
            std::fs::create_dir_all(&dir).unwrap();
            for k in 0..10 {
                std::fs::write(dir.join(format!("f{k}.js")), "x").unwrap();
                std::fs::write(dir.join(format!("f{k}.py")), "x").unwrap();
            }
        }
    }
}

fn naive_walk(
    root: &Path,
    includes: &[globset::GlobMatcher],
    excludes: &[globset::GlobMatcher],
) -> BTreeSet<Twig> {
    let mut result = BTreeSet::new();
    naive_walk_inner(root, root, includes, excludes, &mut result);
    result
}

fn naive_walk_inner(
    root: &Path,
    dir: &Path,
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
            let inc = includes.iter().any(|g| g.is_match(rel));
            let exc = excludes.iter().any(|g| g.is_match(rel));
            if inc && !exc {
                if let Ok(twig) = Twig::new(rel.to_path_buf()) {
                    result.insert(twig);
                }
            }
        } else if path.is_dir() {
            naive_walk_inner(root, &path, includes, excludes, result);
        }
    }
}

fn ignore_walk(root: &Path, overrides: &ignore::overrides::Override) -> BTreeSet<Twig> {
    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .standard_filters(false)
        .overrides(overrides.clone())
        .sort_by_file_path(|a, b| a.cmp(b));
    let walker = builder.build();
    let mut result = BTreeSet::new();
    for entry in walker {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap();
        if let Ok(twig) = Twig::new(rel.to_path_buf()) {
            result.insert(twig);
        }
    }
    result
}

fn ignore_parallel_walk(root: &Path, overrides: &ignore::overrides::Override) -> BTreeSet<Twig> {
    let mut builder = ignore::WalkBuilder::new(root);
    builder.standard_filters(false).overrides(overrides.clone());
    let walker = builder.build_parallel();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut vis_builder = ParVisitorBuilder::new(root, tx);
    walker.visit(&mut vis_builder);
    drop(vis_builder);

    rx.into_iter().collect()
}

struct ParVisitorBuilder {
    root: PathBuf,
    tx: std::sync::mpsc::Sender<Twig>,
}

impl ParVisitorBuilder {
    fn new(root: &Path, tx: std::sync::mpsc::Sender<Twig>) -> Self {
        Self {
            root: root.to_path_buf(),
            tx,
        }
    }
}

impl<'s> ignore::ParallelVisitorBuilder<'s> for ParVisitorBuilder {
    fn build(&mut self) -> Box<dyn ignore::ParallelVisitor + 's> {
        Box::new(ParVisitor {
            root: self.root.clone(),
            tx: self.tx.clone(),
        })
    }
}

struct ParVisitor {
    root: PathBuf,
    tx: std::sync::mpsc::Sender<Twig>,
}

impl ignore::ParallelVisitor for ParVisitor {
    fn visit(&mut self, entry: Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState {
        let Ok(entry) = entry else {
            return ignore::WalkState::Continue;
        };
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            return ignore::WalkState::Continue;
        }
        let Some(rel) = entry.path().strip_prefix(&self.root).ok() else {
            return ignore::WalkState::Continue;
        };
        if let Ok(twig) = Twig::new(rel.to_path_buf()) {
            let _ = self.tx.send(twig);
        }
        ignore::WalkState::Continue
    }
}

fn bough_glob_walk(root: &TestRoot, includes: &[Glob], excludes: &[Glob]) -> BTreeSet<Twig> {
    let mut walker = TwigWalker::new(root);
    for g in includes {
        walker = walker.include(g.clone());
    }
    for g in excludes {
        walker = walker.exclude(g.clone());
    }
    walker.iter().collect()
}

fn build_overrides(
    root: &Path,
    includes: &[&str],
    excludes: &[&str],
) -> ignore::overrides::Override {
    let mut builder = ignore::overrides::OverrideBuilder::new(root);
    for pat in includes {
        builder.add(pat).unwrap();
    }
    for pat in excludes {
        builder.add(&format!("!{pat}")).unwrap();
    }
    builder.build().unwrap()
}

fn build_globset_matchers(pats: &[&str]) -> Vec<globset::GlobMatcher> {
    pats.iter()
        .map(|s| {
            globset::GlobBuilder::new(s)
                .literal_separator(true)
                .build()
                .unwrap()
                .compile_matcher()
        })
        .collect()
}

fn build_globs(pats: &[&str]) -> Vec<Glob> {
    pats.iter().map(|s| Glob::try_from(*s).unwrap()).collect()
}

struct TreeFixture {
    _dir: tempfile::TempDir,
    root: TestRoot,
    path: PathBuf,
}

impl TreeFixture {
    fn new(builder: fn(&Path)) -> Self {
        let dir = tempfile::tempdir().unwrap();
        builder(dir.path());
        let root = TestRoot::new(dir.path());
        let path = dir.path().to_path_buf();
        Self {
            _dir: dir,
            root,
            path,
        }
    }
}

macro_rules! bench_group {
    ($name:ident, $group:expr, $tree:expr, $includes:expr, $excludes:expr) => {
        fn $name(c: &mut Criterion) {
            let fixture = TreeFixture::new($tree);
            let includes: &[&str] = $includes;
            let excludes: &[&str] = $excludes;

            let naive_inc = build_globset_matchers(includes);
            let naive_exc = build_globset_matchers(excludes);
            let overrides = build_overrides(&fixture.path, includes, excludes);
            let bg_inc = build_globs(includes);
            let bg_exc = build_globs(excludes);

            let mut group = c.benchmark_group($group);
            group.bench_function("naive", |b| {
                b.iter(|| naive_walk(&fixture.path, &naive_inc, &naive_exc))
            });
            group.bench_function("ignore", |b| {
                b.iter(|| ignore_walk(&fixture.path, &overrides))
            });
            group.bench_function("ignore_parallel", |b| {
                b.iter(|| ignore_parallel_walk(&fixture.path, &overrides))
            });
            group.bench_function("bough_glob", |b| {
                b.iter(|| bough_glob_walk(&fixture.root, &bg_inc, &bg_exc))
            });
            group.finish();
        }
    };
}

bench_group!(
    bench_deep,
    "deep_tree",
    make_deep_tree,
    &["**/*.js"],
    &["d0/**"]
);
bench_group!(
    bench_wide,
    "wide_tree",
    make_wide_tree,
    &["**/*.js"],
    &["pkg0/**"]
);
bench_group!(
    bench_mixed,
    "mixed_tree",
    make_mixed_tree,
    &["**/*.js"],
    &["area0/**", "area1/**"]
);

fn make_prunable_tree(root: &Path) {
    for dir_name in &[
        "src", "lib", "test", "build", "dist", "vendor", "docs", "scripts", "config", "assets",
    ] {
        for sub in &["a", "b", "c", "d", "e"] {
            let dir = root.join(dir_name).join(sub).join("deep");
            std::fs::create_dir_all(&dir).unwrap();
            for k in 0..20 {
                std::fs::write(dir.join(format!("f{k}.js")), "x").unwrap();
                std::fs::write(dir.join(format!("f{k}.py")), "x").unwrap();
            }
        }
    }
}

bench_group!(
    bench_prunable,
    "prunable_tree",
    make_prunable_tree,
    &["src/**/*.js"],
    &[]
);

criterion_group!(benches, bench_deep, bench_wide, bench_mixed, bench_prunable);
criterion_main!(benches);

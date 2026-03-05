use crate::file::{Error, Root};
use std::path::{Path, PathBuf};

// core[impl file.twig]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Twig(PathBuf);

impl Twig {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if path.is_absolute() {
            return Err(Error::TwigMustBeRelative(path));
        }
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

// core[impl twig.iter.root]
// core[impl twig.iter]
#[derive(Clone, PartialEq)]
pub struct TwigsIterBuilder {
    // core[impl twig.iter.include]
    include: Vec<String>,
    // core[impl twig.iter.exclude]
    exclude: Vec<String>,
}

impl TwigsIterBuilder {
    pub fn new() -> Self {
        Self {
            include: Vec::new(),
            exclude: Vec::new(),
        }
    }

    pub fn with_include_glob(mut self, pattern: &str) -> Self {
        self.include.push(pattern.to_string());
        self
    }

    pub fn with_exclude_glob(mut self, pattern: &str) -> Self {
        self.exclude.push(pattern.to_string());
        self
    }

    // core[impl twig.iter.new]
    pub fn build(self, root: &impl Root) -> TwigsIter {
        let root = root.path().to_path_buf();
        let include = self
            .include
            .iter()
            .filter_map(|p| glob::Pattern::new(&root.join(p).to_string_lossy()).ok())
            .collect();
        let exclude = self
            .exclude
            .iter()
            .filter_map(|p| glob::Pattern::new(&root.join(p).to_string_lossy()).ok())
            .collect();
        let walker = walkdir::WalkDir::new(&root).sort_by_file_name().into_iter();
        TwigsIter {
            root,
            include,
            exclude,
            walker,
        }
    }
}

pub struct TwigsIter {
    root: PathBuf,
    include: Vec<glob::Pattern>,
    exclude: Vec<glob::Pattern>,
    walker: walkdir::IntoIter,
}

impl Iterator for TwigsIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let entry = self.walker.next()?.ok()?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let path_str = path.to_string_lossy();

            // core[impl twig.iter.include.empty]
            // core[impl twig.iter.include.match]
            if !self.include.iter().any(|p| p.matches(&path_str)) {
                continue;
            }

            // core[impl twig.iter.exclude.match]
            if self.exclude.iter().any(|p| p.matches(&path_str)) {
                continue;
            }

            let rel = path.strip_prefix(&self.root).ok()?;
            return Twig::new(rel.to_path_buf()).ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::TestRoot;

    // core[verify file.twig]
    #[test]
    fn twig_accepts_relative_path() {
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        assert_eq!(twig.path(), Path::new("src/main.rs"));
    }

    // core[verify file.twig]
    #[test]
    fn twig_rejects_absolute_path() {
        assert!(matches!(
            Twig::new(PathBuf::from("/absolute/path.rs")),
            Err(Error::TwigMustBeRelative(_))
        ));
    }

    // core[verify twig.iter.root]
    // core[verify twig.iter.new]
    // core[verify twig.iter]
    #[test]
    fn iter_takes_root_and_yields_twigs() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        let twigs: Vec<_> = TwigsIterBuilder::new()
            .with_include_glob("*.txt")
            .build(&root)
            .collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }

    fn make_test_tree(dir: &std::path::Path) {
        for p in &[
            "src/index.js",
            "src/utils.js",
            "src/style.css",
            "test/index.test.js",
            "build/output.js",
            "README.md",
        ] {
            let full = dir.join(p);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, "content").unwrap();
        }
    }

    fn sorted_twigs(iter: TwigsIter) -> Vec<PathBuf> {
        let mut twigs: Vec<PathBuf> = iter.map(|t| t.path().to_path_buf()).collect();
        twigs.sort();
        twigs
    }

    // core[verify twig.iter]
    // core[verify twig.iter.include.match]
    #[test]
    fn iter_includes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("src/**/*.js")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/index.js"), PathBuf::from("src/utils.js")]
        );
    }

    // core[verify twig.iter.include.match]
    #[test]
    fn iter_includes_multiple_globs() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("src/**/*.js")
                .with_include_glob("**/*.md")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
            ]
        );
    }

    // core[verify twig.iter.include.empty]
    #[test]
    fn iter_empty_include_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<_> = TwigsIterBuilder::new().build(&root).collect();
        assert!(twigs.is_empty());
    }

    // core[verify twig.iter.exclude.match]
    #[test]
    fn iter_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs = sorted_twigs(
            TwigsIterBuilder::new()
                .with_include_glob("**/*.js")
                .with_exclude_glob("build/**")
                .build(&root),
        );
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
                PathBuf::from("test/index.test.js"),
            ]
        );
    }
}

use crate::{Glob, TwigWalker};
use bough_fs::{Root, Twig};
use tracing::debug;

#[derive(Clone, PartialEq, Debug)]
pub struct TwigsIterBuilder {
    include: Vec<String>,
    exclude: Vec<String>,
}

impl Default for TwigsIterBuilder {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn build<R: Root>(self, root: &R) -> std::vec::IntoIter<Twig> {
        debug!(
            root = %root.path().display(),
            includes = ?self.include,
            excludes = ?self.exclude,
            "building twigs iterator"
        );

        if self.include.is_empty() {
            return Vec::new().into_iter();
        }

        let mut walker = TwigWalker::new(root);

        for pat in &self.include {
            let glob = Glob::try_from(pat.as_str()).expect("invalid include glob");
            walker = walker.include(glob);
        }

        for pat in &self.exclude {
            let glob = Glob::try_from(pat.as_str()).expect("invalid exclude glob");
            walker = walker.exclude(glob);
        }

        let mut twigs: Vec<Twig> = walker.iter().collect();
        twigs.sort();
        twigs.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_fs::TestRoot;
    use std::path::{Path, PathBuf};

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

    fn make_test_tree(dir: &Path) {
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

    #[test]
    fn iter_includes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<PathBuf> = TwigsIterBuilder::new()
            .with_include_glob("src/**/*.js")
            .build(&root)
            .map(|t| t.path().to_path_buf())
            .collect();
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/index.js"), PathBuf::from("src/utils.js")]
        );
    }

    #[test]
    fn iter_includes_multiple_globs() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<PathBuf> = TwigsIterBuilder::new()
            .with_include_glob("src/**/*.js")
            .with_include_glob("**/*.md")
            .build(&root)
            .map(|t| t.path().to_path_buf())
            .collect();
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
            ]
        );
    }

    #[test]
    fn iter_empty_include_yields_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<_> = TwigsIterBuilder::new().build(&root).collect();
        assert!(twigs.is_empty());
    }

    #[test]
    fn iter_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot::new(dir.path());
        make_test_tree(dir.path());
        let twigs: Vec<PathBuf> = TwigsIterBuilder::new()
            .with_include_glob("**/*.js")
            .with_exclude_glob("build/**")
            .build(&root)
            .map(|t| t.path().to_path_buf())
            .collect();
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

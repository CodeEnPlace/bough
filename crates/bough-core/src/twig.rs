use crate::file::Error;
use std::path::{Path, PathBuf};

// core[impl file.twig]
#[derive(Debug, Clone, PartialEq)]
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
#[derive(Debug, Clone, PartialEq)]
pub struct TwigsIter {
    twigs: Vec<Twig>,
}

impl TwigsIter {
    pub fn new(
        root: &Path,
        include: &[String],
        exclude: &[String],
        ignore_files: &[PathBuf],
    ) -> Self {
        let root = root;

        // core[impl twig.iter.include.match]
        let mut included: Vec<PathBuf> = include
            .iter()
            .filter_map(|pattern| {
                let abs_pattern = root.join(pattern).to_string_lossy().to_string();
                glob::glob(&abs_pattern).ok()
            })
            .flatten()
            .filter_map(|r: Result<PathBuf, _>| r.ok())
            .filter(|p: &PathBuf| p.is_file())
            .collect();

        included.sort();
        included.dedup();

        //
        let ignore_patterns: Vec<glob::Pattern> = ignore_files
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .flat_map(|contents: String| {
                let root = root.to_path_buf();
                contents
                    .lines()
                    .filter(|l: &&str| !l.trim().is_empty() && !l.starts_with('#'))
                    .flat_map(|l: &str| {
                        let trimmed = l.trim().trim_end_matches('/');
                        let with_glob = format!("{trimmed}/**");
                        [trimmed.to_string(), with_glob]
                    })
                    .filter_map(move |entry: String| {
                        let abs = root.join("**").join(&entry).to_string_lossy().to_string();
                        glob::Pattern::new(&abs).ok()
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // core[impl twig.iter.exclude.match]
        let exclude_patterns: Vec<glob::Pattern> = exclude
            .iter()
            .filter_map(|pattern| {
                let abs = root.join(pattern).to_string_lossy().to_string();
                glob::Pattern::new(&abs).ok()
            })
            .collect();

        let twigs: Vec<Twig> = included
            .into_iter()
            .filter(move |path| {
                let path_str = path.to_string_lossy();
                !exclude_patterns.iter().any(|p| p.matches(&path_str))
                    && !ignore_patterns.iter().any(|p| p.matches(&path_str))
            })
            .filter_map(|abs| {
                abs.strip_prefix(root)
                    .ok()
                    .and_then(|rel| Twig::new(rel.to_path_buf()).ok())
            })
            .collect();

        Self { twigs }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Twig> {
        self.twigs.iter()
    }
}

impl Iterator for TwigsIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        if self.twigs.is_empty() {
            None
        } else {
            Some(self.twigs.remove(0))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::Root;

    #[derive(Debug, Clone, PartialEq)]
    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new(path: impl Into<PathBuf>) -> Self {
            Self(path.into())
        }
    }

    impl Root for TestRoot {
        fn path(&self) -> &Path {
            &self.0
        }
    }

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
    fn files_iter_takes_root_and_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        let root = TestRoot::new(dir.path());
        let twigs: Vec<_> = TwigsIter::new(root.path(), &["*.txt".into()], &[], &[]).collect();
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

    fn sorted_twigs(root: &TestRoot, include: &[String], exclude: &[String], ignore_files: &[PathBuf]) -> Vec<PathBuf> {
        let mut twigs: Vec<PathBuf> = TwigsIter::new(root.path(), include, exclude, ignore_files)
            .map(|t| t.path().to_path_buf())
            .collect();
        twigs.sort();
        twigs
    }

    // core[verify twig.iter]
    // core[verify twig.iter.include.match]
    #[test]
    fn iter_includes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let twigs = sorted_twigs(&root, &["src/**/*.js".into()], &[], &[]);
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/index.js"), PathBuf::from("src/utils.js"),]
        );
    }

    // core[verify twig.iter.include.match]
    #[test]
    fn iter_includes_multiple_globs() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let twigs = sorted_twigs(&root, &["src/**/*.js".into(), "**/*.md".into()], &[], &[]);
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
            ]
        );
    }

    // core[verify twig.iter.exclude.match]
    #[test]
    fn iter_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let twigs = sorted_twigs(&root, &["**/*.js".into()], &["build/**".into()], &[]);
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
                PathBuf::from("test/index.test.js"),
            ]
        );
    }

    // removed - vcs-ignore no longer a spec
    #[test]
    fn iter_respects_vcs_ignore() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let ignore_path = dir.path().join(".gitignore");
        std::fs::write(&ignore_path, "build/\n").unwrap();
        let root = TestRoot::new(dir.path());
        let twigs = sorted_twigs(&root, &["**/*.js".into()], &[], &[ignore_path]);
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

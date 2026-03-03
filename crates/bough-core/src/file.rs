use bough_typed_hash::{HashInto, TypedHashable};
use std::path::{Path, PathBuf};

#[derive(bough_typed_hash::TypedHash)]
pub struct FileHash([u8; 32]);

#[derive(Debug)]
pub enum Error {
    RootMustBeAbsolute(PathBuf),
    TwigMustBeRelative(PathBuf),
}

pub trait Root: std::fmt::Debug + Clone + PartialEq {
    fn path(&self) -> &Path;
}

// core[impl file.file]
#[derive(Debug, Clone, PartialEq)]
pub struct File<'a, R: Root> {
    root: &'a R,
    twig: &'a Twig,
}

// core[impl file.twig]
#[derive(Debug, Clone, PartialEq)]
pub struct Twig(PathBuf);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RootMustBeAbsolute(p) => {
                write!(f, "root path must be absolute: {}", p.display())
            }
            Error::TwigMustBeRelative(p) => {
                write!(f, "twig path must be relative: {}", p.display())
            }
        }
    }
}

impl std::error::Error for Error {}

impl<'a, R: Root> File<'a, R> {
    pub fn new(root: &'a R, twig: &'a Twig) -> Self {
        Self { root, twig }
    }

    // core[impl file.file.resolve]
    pub fn resolve(&self) -> PathBuf {
        self.root.path().join(&self.twig.0)
    }

    // core[impl file.transplant]
    pub fn transplant<'b, S: Root>(&self, root: &'b S) -> File<'b, S>
    where
        'a: 'b,
    {
        File {
            root,
            twig: self.twig,
        }
    }
}

// core[impl file.file.hash]
impl<R: Root> HashInto for File<'_, R> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.resolve().hash_into(state)
    }
}

impl<R: Root> TypedHashable for File<'_, R> {
    type Hash = FileHash;
}

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

// core[impl file.root]
pub(crate) fn validate_root(path: &PathBuf) -> Result<(), Error> {
    if !path.is_absolute() {
        return Err(Error::RootMustBeAbsolute(path.clone()));
    }
    Ok(())
}

// core[impl file.files.config]
// core[impl file.files.root]
// core[impl file.files.iter]
pub struct FilesIter {
    twigs: std::vec::IntoIter<Twig>,
}

impl FilesIter {
    pub fn new<R: Root>(root: &R, config: &crate::config::FileSourceConfig) -> Self {
        let root = root.path();

        // core[impl file.files.iter.include]
        let mut included: Vec<PathBuf> = config
            .include
            .iter()
            .filter_map(|pattern| {
                let abs_pattern = root.join(pattern).to_string_lossy().to_string();
                glob::glob(&abs_pattern).ok()
            })
            .flatten()
            .filter_map(|r| r.ok())
            .filter(|p| p.is_file())
            .collect();

        included.sort();
        included.dedup();

        // core[impl file.files.iter.vcs-ignore]
        let ignore_patterns: Vec<glob::Pattern> = config
            .ignore_files
            .iter()
            .filter_map(|path| std::fs::read_to_string(path).ok())
            .flat_map(|contents| {
                let root = root.to_path_buf();
                contents
                    .lines()
                    .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                    .flat_map(|l| {
                        let trimmed = l.trim().trim_end_matches('/');
                        let with_glob = format!("{trimmed}/**");
                        [trimmed.to_string(), with_glob]
                    })
                    .filter_map(move |entry| {
                        let abs = root.join("**").join(&entry).to_string_lossy().to_string();
                        glob::Pattern::new(&abs).ok()
                    })
                    .collect::<Vec<_>>()
            })
            .collect();

        // core[impl file.files.iter.exclude]
        let exclude_patterns: Vec<glob::Pattern> = config
            .exclude
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

        Self {
            twigs: twigs.into_iter(),
        }
    }
}

impl Iterator for FilesIter {
    type Item = Twig;

    fn next(&mut self) -> Option<Self::Item> {
        self.twigs.next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_typed_hash::sha2::Digest;

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

    // core[verify file.files.config]
    // core[verify file.files.root]
    // core[verify file.files.iter]
    #[test]
    fn files_iter_takes_root_and_config() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();
        let root = TestRoot::new(dir.path());
        let config = crate::config::FileSourceConfig {
            include: vec!["*.txt".into()],
            ..Default::default()
        };
        let twigs: Vec<_> = FilesIter::new(&root, &config).collect();
        assert_eq!(twigs.len(), 1);
        assert_eq!(twigs[0].path(), Path::new("a.txt"));
    }

    // core[verify file.root]
    #[test]
    fn validate_root_accepts_absolute_path() {
        assert!(validate_root(&PathBuf::from("/tmp/project")).is_ok());
    }

    // core[verify file.root]
    #[test]
    fn validate_root_rejects_relative_path() {
        assert!(matches!(
            validate_root(&PathBuf::from("relative/path")),
            Err(Error::RootMustBeAbsolute(_))
        ));
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

    // core[verify file.file]
    #[test]
    fn file_holds_root_and_twig() {
        let root = TestRoot::new("/tmp/project");
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.root.path(), root.path());
        assert_eq!(file.twig.path(), twig.path());
    }

    // core[verify file.file.resolve]
    #[test]
    fn file_resolve_joins_root_and_twig() {
        let root = TestRoot::new("/tmp/project");
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.resolve(), PathBuf::from("/tmp/project/src/main.rs"));
    }

    // core[verify file.file.hash]
    #[test]
    fn file_hash_reads_resolved_contents() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/test.txt"), "hello").unwrap();

        let root = TestRoot::new(dir.path());
        let twig = Twig::new(PathBuf::from("src/test.txt")).unwrap();
        let file = File::new(&root, &twig);

        let mut state = bough_typed_hash::ShaState::new();
        file.hash_into(&mut state).unwrap();
    }

    // core[verify file.file.hash]
    #[test]
    fn file_hash_fails_for_missing_file() {
        let root = TestRoot::new("/tmp/nonexistent_dir_abc123");
        let twig = Twig::new(PathBuf::from("missing.txt")).unwrap();
        let file = File::new(&root, &twig);

        let mut state = bough_typed_hash::ShaState::new();
        assert!(file.hash_into(&mut state).is_err());
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

    fn sorted_twigs(root: &TestRoot, config: &crate::config::FileSourceConfig) -> Vec<PathBuf> {
        let mut twigs: Vec<PathBuf> = FilesIter::new(root, config)
            .map(|t| t.path().to_path_buf())
            .collect();
        twigs.sort();
        twigs
    }

    // core[verify file.files.iter]
    // core[verify file.files.iter.include]
    #[test]
    fn iter_includes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let config = crate::config::FileSourceConfig {
            include: vec!["src/**/*.js".into()],
            ..Default::default()
        };
        let twigs = sorted_twigs(&root, &config);
        assert_eq!(
            twigs,
            vec![PathBuf::from("src/index.js"), PathBuf::from("src/utils.js"),]
        );
    }

    // core[verify file.files.iter.include]
    #[test]
    fn iter_includes_multiple_globs() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let config = crate::config::FileSourceConfig {
            include: vec!["src/**/*.js".into(), "**/*.md".into()],
            ..Default::default()
        };
        let twigs = sorted_twigs(&root, &config);
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("README.md"),
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
            ]
        );
    }

    // core[verify file.files.iter.exclude]
    #[test]
    fn iter_excludes_matching_files() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let root = TestRoot::new(dir.path());
        let config = crate::config::FileSourceConfig {
            include: vec!["**/*.js".into()],
            exclude: vec!["build/**".into()],
            ..Default::default()
        };
        let twigs = sorted_twigs(&root, &config);
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
                PathBuf::from("test/index.test.js"),
            ]
        );
    }

    // core[verify file.files.iter.vcs-ignore]
    #[test]
    fn iter_respects_vcs_ignore() {
        let dir = tempfile::tempdir().unwrap();
        make_test_tree(dir.path());
        let ignore_path = dir.path().join(".gitignore");
        std::fs::write(&ignore_path, "build/\n").unwrap();
        let root = TestRoot::new(dir.path());
        let config = crate::config::FileSourceConfig {
            include: vec!["**/*.js".into()],
            ignore_files: vec![ignore_path],
            ..Default::default()
        };
        let twigs = sorted_twigs(&root, &config);
        assert_eq!(
            twigs,
            vec![
                PathBuf::from("src/index.js"),
                PathBuf::from("src/utils.js"),
                PathBuf::from("test/index.test.js"),
            ]
        );
    }

    // core[verify file.transplant]
    #[test]
    fn transplant_replaces_root() {
        let root_a = TestRoot::new("/project/a");
        let root_b = TestRoot::new("/project/b");
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root_a, &twig);
        let moved = file.transplant(&root_b);
        assert_eq!(moved.resolve(), PathBuf::from("/project/b/src/main.rs"));
    }
}

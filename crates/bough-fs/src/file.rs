pub use crate::twig::Twig;
use bough_typed_hash::{HashInto, TypedHashable};
use std::path::{Path, PathBuf};
use tracing::trace;

#[derive(bough_typed_hash::TypedHash)]
pub struct FileHash([u8; 32]);

#[derive(Debug)]
pub enum Error {
    RootMustBeAbsolute(PathBuf),
    TwigMustBeRelative(PathBuf),
    TwigNotUtf8(PathBuf),
}

pub trait Root: std::fmt::Debug + Clone + PartialEq {
    fn path(&self) -> &Path;
}

#[derive(Debug, Clone, PartialEq)]
pub struct File<'a, R: Root> {
    root: &'a R,
    twig: &'a Twig,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::RootMustBeAbsolute(p) => {
                write!(f, "root path must be absolute: {}", p.display())
            }
            Error::TwigMustBeRelative(p) => {
                write!(f, "twig path must be relative: {}", p.display())
            }
            Error::TwigNotUtf8(p) => {
                write!(f, "twig path must be valid UTF-8: {}", p.display())
            }
        }
    }
}

impl std::error::Error for Error {}

impl<'a, R: Root> File<'a, R> {
    pub fn new(root: &'a R, twig: &'a Twig) -> Self {
        Self { root, twig }
    }

    pub fn resolve(&self) -> PathBuf {
        let resolved = self.root.path().join(self.twig.path());
        trace!(path = %resolved.display(), "resolved file path");
        resolved
    }

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

impl<R: Root> HashInto for File<'_, R> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.resolve().hash_into(state)
    }
}

impl<R: Root> TypedHashable for File<'_, R> {
    type Hash = FileHash;
}

pub fn validate_root(path: &PathBuf) -> Result<(), Error> {
    if !path.is_absolute() {
        return Err(Error::RootMustBeAbsolute(path.clone()));
    }
    Ok(())
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Debug, Clone, PartialEq)]
pub struct TestRoot(PathBuf);

#[cfg(any(test, feature = "test-support"))]
impl TestRoot {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
}

#[cfg(any(test, feature = "test-support"))]
impl Root for TestRoot {
    fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_typed_hash::sha2::Digest;

    use super::TestRoot;

    #[test]
    fn validate_root_accepts_absolute_path() {
        assert!(validate_root(&PathBuf::from("/tmp/project")).is_ok());
    }

    #[test]
    fn validate_root_rejects_relative_path() {
        assert!(matches!(
            validate_root(&PathBuf::from("relative/path")),
            Err(Error::RootMustBeAbsolute(_))
        ));
    }

    #[test]
    fn file_holds_root_and_twig() {
        let root = TestRoot::new("/tmp/project");
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.root.path(), root.path());
        assert_eq!(file.twig.path(), twig.path());
    }

    #[test]
    fn file_resolve_joins_root_and_twig() {
        let root = TestRoot::new("/tmp/project");
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.resolve(), PathBuf::from("/tmp/project/src/main.rs"));
    }

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

    #[test]
    fn file_hash_fails_for_missing_file() {
        let root = TestRoot::new("/tmp/nonexistent_dir_abc123");
        let twig = Twig::new(PathBuf::from("missing.txt")).unwrap();
        let file = File::new(&root, &twig);

        let mut state = bough_typed_hash::ShaState::new();
        assert!(file.hash_into(&mut state).is_err());
    }

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

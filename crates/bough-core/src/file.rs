pub use crate::twig::Twig;
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
        self.root.path().join(self.twig.path())
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

// core[impl file.root]
pub(crate) fn validate_root(path: &PathBuf) -> Result<(), Error> {
    if !path.is_absolute() {
        return Err(Error::RootMustBeAbsolute(path.clone()));
    }
    Ok(())
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

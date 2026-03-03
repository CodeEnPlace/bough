use bough_typed_hash::{HashInto, TypedHashable};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(bough_typed_hash::TypedHash)]
pub struct FileHash([u8; 32]);

#[derive(Debug)]
pub enum Error {
    RootMustBeAbsolute(PathBuf),
    TwigMustBeRelative(PathBuf),
}

// core[impl file.file]
#[derive(Debug, Clone, PartialEq)]
pub struct File<'a> {
    root: &'a Root,
    twig: &'a Twig,
}

// core[impl file.root]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Root(PathBuf);

// core[impl file.twig]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl<'a> File<'a> {
    pub fn new(root: &'a Root, twig: &'a Twig) -> Self {
        Self { root, twig }
    }

    // core[impl file.file.resolve]
    pub fn resolve(&self) -> PathBuf {
        self.root.0.join(&self.twig.0)
    }

    // core[impl file.transplant]
    pub fn transplant(&self, root: &'a Root) -> Self {
        Self {
            root,
            twig: self.twig,
        }
    }
}

// core[impl file.file.hash]
impl HashInto for File<'_> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.resolve().hash_into(state)
    }
}

impl TypedHashable for File<'_> {
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

impl Root {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        if !path.is_absolute() {
            return Err(Error::RootMustBeAbsolute(path));
        }
        Ok(Self(path))
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // core[verify file.root]
    #[test]
    fn root_accepts_absolute_path() {
        let root = Root::new(PathBuf::from("/tmp/project")).unwrap();
        assert_eq!(root.path(), Path::new("/tmp/project"));
    }

    // core[verify file.root]
    #[test]
    fn root_rejects_relative_path() {
        assert!(matches!(
            Root::new(PathBuf::from("relative/path")),
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
        let root = Root::new(PathBuf::from("/tmp/project")).unwrap();
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.root.path(), root.path());
        assert_eq!(file.twig.path(), twig.path());
    }

    // core[verify file.file.resolve]
    #[test]
    fn file_resolve_joins_root_and_twig() {
        let root = Root::new(PathBuf::from("/tmp/project")).unwrap();
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root, &twig);
        assert_eq!(file.resolve(), PathBuf::from("/tmp/project/src/main.rs"));
    }

    // core[verify file.file.hash]
    #[test]
    fn file_hash_reads_resolved_contents() {
        use bough_typed_hash::sha2::Digest;

        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/test.txt"), "hello").unwrap();

        let root = Root::new(dir.path().to_path_buf()).unwrap();
        let twig = Twig::new(PathBuf::from("src/test.txt")).unwrap();
        let file = File::new(&root, &twig);

        let mut state = bough_typed_hash::ShaState::new();
        file.hash_into(&mut state).unwrap();
    }

    // core[verify file.file.hash]
    #[test]
    fn file_hash_fails_for_missing_file() {
        use bough_typed_hash::sha2::Digest;

        let root = Root::new(PathBuf::from("/tmp/nonexistent_dir_abc123")).unwrap();
        let twig = Twig::new(PathBuf::from("missing.txt")).unwrap();
        let file = File::new(&root, &twig);

        let mut state = bough_typed_hash::ShaState::new();
        assert!(file.hash_into(&mut state).is_err());
    }

    // core[verify file.transplant]
    #[test]
    fn transplant_replaces_root() {
        let root_a = Root::new(PathBuf::from("/project/a")).unwrap();
        let root_b = Root::new(PathBuf::from("/project/b")).unwrap();
        let twig = Twig::new(PathBuf::from("src/main.rs")).unwrap();
        let file = File::new(&root_a, &twig);
        let moved = file.transplant(&root_b);
        assert_eq!(moved.resolve(), PathBuf::from("/project/b/src/main.rs"));
    }
}

use std::path::{Path, PathBuf};

use crate::file::{Root, Twig};

// core[impl phase.root]
// core[impl phase.pwd]
pub struct Phase<'a, R: Root> {
    root: &'a R,
    pwd: Twig,
}

impl<'a, R: Root> Phase<'a, R> {
    pub fn root(&self) -> &R {
        self.root
    }

    pub fn pwd(&self) -> &Twig {
        &self.pwd
    }
}

pub struct PhaseOutcome {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestRoot(PathBuf);
    impl Root for TestRoot {
        fn path(&self) -> &Path {
            &self.0
        }
    }

    // core[verify phase.root]
    #[test]
    fn phase_holds_root() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let pwd = crate::file::Twig::new(PathBuf::from("src")).unwrap();
        let phase = Phase { root: &root, pwd };
        assert_eq!(phase.root().path(), Path::new("/tmp/project"));
    }

    // core[verify phase.pwd]
    #[test]
    fn phase_holds_pwd_twig() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let pwd = crate::file::Twig::new(PathBuf::from("src/test")).unwrap();
        let phase = Phase { root: &root, pwd };
        assert_eq!(phase.pwd().path(), Path::new("src/test"));
    }
}

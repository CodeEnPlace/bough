use std::path::{Path, PathBuf};

use crate::file::Root;

// core[impl phase.root]
pub struct Phase<'a, R: Root> {
    root: &'a R,
}

impl<'a, R: Root> Phase<'a, R> {
    pub fn root(&self) -> &R {
        self.root
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
        let phase = Phase { root: &root };
        assert_eq!(phase.root().path(), Path::new("/tmp/project"));
    }
}

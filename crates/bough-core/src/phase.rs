use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::file::{Root, Twig};

// core[impl phase.root]
// core[impl phase.pwd]
// core[impl phase.env]
// core[impl phase.cmd]
pub struct Phase<'a, R: Root> {
    root: &'a R,
    pwd: Twig,
    env: HashMap<String, String>,
    cmd: Vec<String>,
}

impl<'a, R: Root> Phase<'a, R> {
    pub fn root(&self) -> &R {
        self.root
    }

    pub fn pwd(&self) -> &Twig {
        &self.pwd
    }

    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    pub fn cmd(&self) -> &[String] {
        &self.cmd
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

    fn make_phase<'a>(root: &'a TestRoot) -> Phase<'a, TestRoot> {
        Phase {
            root,
            pwd: crate::file::Twig::new(PathBuf::from("src")).unwrap(),
            env: HashMap::new(),
            cmd: vec!["echo".into(), "hello".into()],
        }
    }

    // core[verify phase.root]
    #[test]
    fn phase_holds_root() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let phase = make_phase(&root);
        assert_eq!(phase.root().path(), Path::new("/tmp/project"));
    }

    // core[verify phase.pwd]
    #[test]
    fn phase_holds_pwd_twig() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let pwd = crate::file::Twig::new(PathBuf::from("src/test")).unwrap();
        let phase = Phase { pwd, ..make_phase(&root) };
        assert_eq!(phase.pwd().path(), Path::new("src/test"));
    }

    // core[verify phase.env]
    #[test]
    fn phase_holds_env_vars() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let env = HashMap::from([("NODE_ENV".into(), "test".into())]);
        let phase = Phase { env, ..make_phase(&root) };
        assert_eq!(phase.env()["NODE_ENV"], "test");
    }

    // core[verify phase.cmd]
    #[test]
    fn phase_holds_cmd() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let cmd = vec!["npx".into(), "vitest".into(), "run".into()];
        let phase = Phase { cmd, ..make_phase(&root) };
        assert_eq!(phase.cmd(), &["npx", "vitest", "run"]);
    }
}

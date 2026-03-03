use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::config::TimeoutConfig;
use crate::file::{File, Root, Twig};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    EmptyCommand,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "phase io error: {e}"),
            Error::EmptyCommand => write!(f, "phase command is empty"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

// core[impl phase.root]
// core[impl phase.pwd]
// core[impl phase.env]
// core[impl phase.cmd]
// core[impl phase.timeout]
pub struct Phase<'a, R: Root> {
    root: &'a R,
    pwd: Twig,
    env: HashMap<String, String>,
    cmd: Vec<String>,
    timeout: TimeoutConfig,
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

    pub fn timeout(&self) -> &TimeoutConfig {
        &self.timeout
    }

    // core[impl phase.run]
    // core[impl phase.run.pwd]
    // core[impl phase.run.env]
    pub fn run(&self) -> Result<PhaseOutcome, Error> {
        if self.cmd.is_empty() {
            return Err(Error::EmptyCommand);
        }

        let working_dir = File::new(self.root, &self.pwd).resolve();

        let start = std::time::Instant::now();
        let output = std::process::Command::new(&self.cmd[0])
            .args(&self.cmd[1..])
            .current_dir(&working_dir)
            .envs(&self.env)
            .output()?;
        let duration = start.elapsed();

        Ok(PhaseOutcome {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }
}

// core[impl phase.out]
// core[impl phase.out.stdio]
// core[impl phase.out.exit]
// core[impl phase.out.duration]
#[derive(Debug)]
pub struct PhaseOutcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
    duration: std::time::Duration,
}

impl PhaseOutcome {
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    pub fn duration(&self) -> std::time::Duration {
        self.duration
    }
}

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
            timeout: TimeoutConfig::default(),
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

    // core[verify phase.timeout]
    #[test]
    fn phase_holds_timeout() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let timeout = TimeoutConfig { absolute: Some(30), relative: Some(3) };
        let phase = Phase { timeout, ..make_phase(&root) };
        assert_eq!(phase.timeout().absolute, Some(30));
        assert_eq!(phase.timeout().relative, Some(3));
    }

    // core[verify phase.out]
    // core[verify phase.out.stdio]
    // core[verify phase.out.exit]
    // core[verify phase.out.duration]
    #[test]
    fn phase_outcome_holds_all_fields() {
        let outcome = PhaseOutcome {
            stdout: b"hello\n".to_vec(),
            stderr: b"warn\n".to_vec(),
            exit_code: 0,
            duration: std::time::Duration::from_millis(150),
        };
        assert_eq!(outcome.stdout(), b"hello\n");
        assert_eq!(outcome.stderr(), b"warn\n");
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(outcome.duration(), std::time::Duration::from_millis(150));
    }

    // core[verify phase.out.exit]
    #[test]
    fn phase_outcome_nonzero_exit_is_not_error() {
        let outcome = PhaseOutcome {
            stdout: vec![],
            stderr: b"error\n".to_vec(),
            exit_code: 1,
            duration: std::time::Duration::from_millis(50),
        };
        assert_eq!(outcome.exit_code(), 1);
    }

    // core[verify phase.run]
    #[test]
    fn phase_run_executes_command() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "hello".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(outcome.exit_code(), 0);
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "hello");
    }

    // core[verify phase.run]
    // core[verify phase.out.exit]
    #[test]
    fn phase_run_nonzero_exit_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "exit 42".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(outcome.exit_code(), 42);
    }

    // core[verify phase.run.pwd]
    #[test]
    fn phase_run_uses_pwd() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["pwd".into()],
            pwd: crate::file::Twig::new(PathBuf::from("subdir")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        assert!(out.trim().ends_with("subdir"), "pwd should end with subdir, got: {out}");
    }

    // core[verify phase.run.env]
    #[test]
    fn phase_run_applies_env() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "echo $MY_VAR".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            env: HashMap::from([("MY_VAR".into(), "hello_env".into())]),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "hello_env");
    }

    // core[verify phase.out.stdio]
    #[test]
    fn phase_run_captures_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "echo err >&2".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(String::from_utf8_lossy(outcome.stderr()).trim(), "err");
    }

    // core[verify phase.out.duration]
    #[test]
    fn phase_run_records_duration() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "0.05".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(outcome.duration() >= std::time::Duration::from_millis(40));
    }
}

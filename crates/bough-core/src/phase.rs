use std::collections::HashMap;
#[cfg(test)]
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
    // core[impl phase.run.timeout]
    // core[impl phase.run.timeout.absolute]
    // core[impl phase.run.timeout.relative]
    pub fn run(&self, reference_duration: Option<std::time::Duration>) -> Result<PhaseOutcome, Error> {
        use wait_timeout::ChildExt;

        if self.cmd.is_empty() {
            return Err(Error::EmptyCommand);
        }

        let working_dir = File::new(self.root, &self.pwd).resolve();

        let effective_timeout = self.effective_timeout(reference_duration);

        let start = std::time::Instant::now();
        let mut child = std::process::Command::new(&self.cmd[0])
            .args(&self.cmd[1..])
            .current_dir(&working_dir)
            .envs(&self.env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        let timed_out = if let Some(timeout) = effective_timeout {
            match child.wait_timeout(timeout)? {
                Some(_status) => false,
                None => {
                    child.kill()?;
                    child.wait()?;
                    true
                }
            }
        } else {
            child.wait()?;
            false
        };

        let duration = start.elapsed();

        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        if let Some(mut out) = child.stdout.take() {
            std::io::Read::read_to_end(&mut out, &mut stdout)?;
        }
        if let Some(mut err) = child.stderr.take() {
            std::io::Read::read_to_end(&mut err, &mut stderr)?;
        }

        let exit_code = if timed_out {
            -1
        } else {
            child.try_wait()?.and_then(|s| s.code()).unwrap_or(-1)
        };

        Ok(PhaseOutcome {
            stdout,
            stderr,
            exit_code,
            duration,
            timed_out,
        })
    }

    fn effective_timeout(&self, reference_duration: Option<std::time::Duration>) -> Option<std::time::Duration> {
        let absolute = self.timeout.absolute.map(std::time::Duration::from_secs);
        let relative = match (self.timeout.relative, reference_duration) {
            (Some(multiplier), Some(ref_dur)) => Some(ref_dur * multiplier as u32),
            _ => None,
        };
        match (absolute, relative) {
            (Some(a), Some(r)) => Some(a.min(r)),
            (Some(a), None) => Some(a),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        }
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
    timed_out: bool,
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

    pub fn timed_out(&self) -> bool {
        self.timed_out
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
            timed_out: false,
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
            timed_out: false,
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
        let outcome = phase.run(None).unwrap();
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
        let outcome = phase.run(None).unwrap();
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
        let outcome = phase.run(None).unwrap();
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
        let outcome = phase.run(None).unwrap();
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
        let outcome = phase.run(None).unwrap();
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
        let outcome = phase.run(None).unwrap();
        assert!(outcome.duration() >= std::time::Duration::from_millis(40));
    }

    // core[verify phase.run.timeout.absolute]
    #[test]
    fn phase_run_kills_on_absolute_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "10".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: TimeoutConfig { absolute: Some(1), relative: None },
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(outcome.timed_out());
        assert!(outcome.duration() < std::time::Duration::from_secs(5));
    }

    // core[verify phase.run.timeout.absolute]
    #[test]
    fn phase_run_no_timeout_when_command_finishes_in_time() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "fast".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: TimeoutConfig { absolute: Some(10), relative: None },
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert_eq!(outcome.exit_code(), 0);
    }

    // core[verify phase.run.timeout.relative]
    #[test]
    fn phase_run_kills_on_relative_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "10".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: TimeoutConfig { absolute: None, relative: Some(2) },
            ..make_phase(&root)
        };
        let ref_dur = std::time::Duration::from_millis(500);
        let outcome = phase.run(Some(ref_dur)).unwrap();
        assert!(outcome.timed_out());
        assert!(outcome.duration() < std::time::Duration::from_secs(5));
    }

    // core[verify phase.run.timeout.relative]
    #[test]
    fn phase_run_relative_timeout_ignored_without_reference() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "ok".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: TimeoutConfig { absolute: None, relative: Some(2) },
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert_eq!(outcome.exit_code(), 0);
    }
}

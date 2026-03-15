#[cfg(test)]
use std::path::{Path, PathBuf};
use std::{collections::HashMap, time::Duration};

use crate::file::{File, Root, Twig};
use tracing::{debug, info, warn};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    EmptyCommand,
    File(crate::file::Error),
    AbsolutePwd(std::path::PathBuf),
    InvalidTimeout,
    NoCmdConfigured,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => write!(f, "phase io error: {e}"),
            Error::EmptyCommand => write!(f, "phase command is empty"),
            Error::File(e) => write!(f, "phase file error: {e}"),
            Error::AbsolutePwd(p) => write!(f, "phase pwd must be relative: {}", p.display()),
            Error::InvalidTimeout => write!(f, "invalid timeout duration"),
            Error::NoCmdConfigured => write!(f, "no command configured for phase"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<crate::file::Error> for Error {
    fn from(e: crate::file::Error) -> Self {
        Error::File(e)
    }
}

fn kill_process_tree(child: &std::process::Child) {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}

// bough[impl phase.root]
// bough[impl phase.pwd]
// bough[impl phase.env]
// bough[impl phase.cmd]
// bough[impl phase.timeout]
pub struct Phase<'a, R: Root> {
    root: &'a R,
    pwd: Twig,
    env: HashMap<String, String>,
    cmd: Vec<String>,
    timeout_absolute: Option<Duration>,
    timeout_relative: Option<f64>,
}

impl<'a, R: Root> Phase<'a, R> {
    pub fn new(
        root: &'a R,
        pwd: Twig,
        env: HashMap<String, String>,
        cmd: Vec<String>,
        timeout_absolute: Option<Duration>,
        timeout_relative: Option<f64>,
    ) -> Self {
        Self {
            root,
            pwd,
            env,
            cmd,
            timeout_absolute,
            timeout_relative,
        }
    }

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

    pub fn timeout_absolute(&self) -> Option<Duration> {
        self.timeout_absolute
    }

    pub fn timeout_relative(&self) -> Option<f64> {
        self.timeout_relative
    }

    // bough[impl phase.run]
    // bough[impl phase.run.pwd]
    // bough[impl phase.run.env]
    // bough[impl phase.run.timeout]
    // bough[impl phase.run.timeout.absolute]
    // bough[impl phase.run.timeout.relative]
    pub fn run(
        &self,
        reference_duration: Option<std::time::Duration>,
    ) -> Result<PhaseOutcome, Error> {
        use wait_timeout::ChildExt;

        if self.cmd.is_empty() {
            warn!("attempted to run phase with empty command");
            return Err(Error::EmptyCommand);
        }

        let working_dir = File::new(self.root, &self.pwd).resolve();
        let effective_timeout = self.effective_timeout(reference_duration);

        info!(
            cmd = ?self.cmd,
            pwd = %working_dir.display(),
            timeout = ?effective_timeout,
            "running phase"
        );

        let start = std::time::Instant::now();
        let mut cmd = std::process::Command::new(&self.cmd[0]);
        cmd.args(&self.cmd[1..])
            .current_dir(&working_dir)
            .envs(&self.env)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }

        let mut child = cmd.spawn()?;

        let stdout_handle = child.stdout.take().map(|out| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut out = out;
                std::io::Read::read_to_end(&mut out, &mut buf).map(|_| buf)
            })
        });
        let stderr_handle = child.stderr.take().map(|err| {
            std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut err = err;
                std::io::Read::read_to_end(&mut err, &mut buf).map(|_| buf)
            })
        });

        let timed_out = if let Some(timeout) = effective_timeout {
            match child.wait_timeout(timeout)? {
                Some(_status) => false,
                None => {
                    kill_process_tree(&child);
                    child.wait()?;
                    true
                }
            }
        } else {
            child.wait()?;
            false
        };

        let duration = start.elapsed();

        let stdout = stdout_handle
            .map(|h| h.join().expect("stdout reader panicked"))
            .transpose()?
            .unwrap_or_default();
        let stderr = stderr_handle
            .map(|h| h.join().expect("stderr reader panicked"))
            .transpose()?
            .unwrap_or_default();

        let exit_code = if timed_out {
            -1
        } else {
            child.try_wait()?.and_then(|s| s.code()).unwrap_or(-1)
        };

        debug!(
            exit_code,
            timed_out,
            duration_ms = duration.as_millis() as u64,
            "phase completed"
        );

        Ok(PhaseOutcome {
            stdout,
            stderr,
            exit_code,
            duration,
            timed_out,
        })
    }

    fn effective_timeout(
        &self,
        reference_duration: Option<std::time::Duration>,
    ) -> Option<std::time::Duration> {
        let absolute = self.timeout_absolute;
        let relative = match (self.timeout_relative, reference_duration) {
            (Some(multiplier), Some(ref_dur)) => Some(ref_dur * multiplier as u32),
            _ => None,
        };
        match (absolute, relative) {
            (Some(a), Some(r)) => Some(std::time::Duration::min(a, r)),
            (Some(a), None) => Some(a),
            (None, Some(r)) => Some(r),
            (None, None) => None,
        }
    }
}

// bough[impl phase.out]
// bough[impl phase.out.stdio]
// bough[impl phase.out.exit]
// bough[impl phase.out.duration]
#[derive(Debug)]
pub struct PhaseOutcome {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
    duration: std::time::Duration,
    timed_out: bool,
}

impl PhaseOutcome {
    #[doc(hidden)]
    pub fn new_for_test(exit_code: i32, duration: std::time::Duration, timed_out: bool, stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self { stdout, stderr, exit_code, duration, timed_out }
    }

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

pub fn run_phase<R: Root>(
    root: &R,
    cmd: &str,
    pwd: std::path::PathBuf,
    env: HashMap<String, String>,
    timeout_absolute: Option<chrono::Duration>,
    timeout_relative: Option<f64>,
    reference_duration: Option<Duration>,
) -> Result<PhaseOutcome, Error> {
    if pwd.is_absolute() {
        return Err(Error::AbsolutePwd(pwd));
    }
    let twig = Twig::new(pwd).map_err(crate::file::Error::from)?;
    let cmd_parts: Vec<String> = cmd.split_whitespace().map(String::from).collect();
    let timeout_abs = timeout_absolute
        .map(|d| d.to_std().map_err(|_| Error::InvalidTimeout))
        .transpose()?;
    let phase = Phase::new(root, twig, env, cmd_parts, timeout_abs, timeout_relative);
    phase.run(reference_duration)
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
            timeout_absolute: None,
            timeout_relative: None,
        }
    }

    // bough[verify phase.root]
    #[test]
    fn phase_holds_root() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let phase = make_phase(&root);
        assert_eq!(phase.root().path(), Path::new("/tmp/project"));
    }

    // bough[verify phase.pwd]
    #[test]
    fn phase_holds_pwd_twig() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let pwd = crate::file::Twig::new(PathBuf::from("src/test")).unwrap();
        let phase = Phase {
            pwd,
            ..make_phase(&root)
        };
        assert_eq!(phase.pwd().path(), Path::new("src/test"));
    }

    // bough[verify phase.env]
    #[test]
    fn phase_holds_env_vars() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let env = HashMap::from([("NODE_ENV".into(), "test".into())]);
        let phase = Phase {
            env,
            ..make_phase(&root)
        };
        assert_eq!(phase.env()["NODE_ENV"], "test");
    }

    // bough[verify phase.cmd]
    #[test]
    fn phase_holds_cmd() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let cmd = vec!["npx".into(), "vitest".into(), "run".into()];
        let phase = Phase {
            cmd,
            ..make_phase(&root)
        };
        assert_eq!(phase.cmd(), &["npx", "vitest", "run"]);
    }

    // bough[verify phase.timeout]
    #[test]
    fn phase_holds_timeout() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let phase = Phase {
            timeout_absolute: Some(Duration::from_secs(30)),
            timeout_relative: Some(3.0),
            ..make_phase(&root)
        };
        assert_eq!(phase.timeout_absolute(), Some(Duration::from_secs(30)));
        assert_eq!(phase.timeout_relative(), Some(3.0));
    }

    // bough[verify phase.out]
    // bough[verify phase.out.stdio]
    // bough[verify phase.out.exit]
    // bough[verify phase.out.duration]
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

    // bough[verify phase.out.exit]
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

    // bough[verify phase.run]
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

    // bough[verify phase.run]
    // bough[verify phase.out.exit]
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

    // bough[verify phase.run.pwd]
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
        assert!(
            out.trim().ends_with("subdir"),
            "pwd should end with subdir, got: {out}"
        );
    }

    // bough[verify phase.run.env]
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
        assert_eq!(
            String::from_utf8_lossy(outcome.stdout()).trim(),
            "hello_env"
        );
    }

    // bough[verify phase.out.stdio]
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

    // bough[verify phase.out.duration]
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

    // bough[verify phase.run.timeout]
    // bough[verify phase.run.timeout.absolute]
    #[test]
    fn phase_run_kills_on_absolute_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "10".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: Some(Duration::from_millis(100)),
            timeout_relative: None,
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(outcome.timed_out());
        assert!(outcome.duration() < std::time::Duration::from_secs(5));
    }

    // bough[verify phase.run.timeout.absolute]
    #[test]
    fn phase_run_no_timeout_when_command_finishes_in_time() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "fast".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: Some(Duration::from_millis(100)),
            timeout_relative: None,
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert_eq!(outcome.exit_code(), 0);
    }

    // bough[verify phase.run.timeout.relative]
    #[test]
    fn phase_run_kills_on_relative_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "10".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: None,
            timeout_relative: Some(2.0),
            ..make_phase(&root)
        };
        let ref_dur = std::time::Duration::from_millis(50);
        let outcome = phase.run(Some(ref_dur)).unwrap();
        assert!(outcome.timed_out());
        assert!(outcome.duration() < std::time::Duration::from_secs(5));
    }

    // bough[verify phase.run.timeout.relative]
    #[test]
    fn phase_run_relative_timeout_ignored_without_reference() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "ok".into()],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: None,
            timeout_relative: Some(2.0),
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert_eq!(outcome.exit_code(), 0);
    }

    fn helper_bin() -> PathBuf {
        let test_exe = std::env::current_exe().expect("current_exe");
        let dir = test_exe.parent().unwrap().parent().unwrap();
        dir.join("bough-test-helper")
    }

    #[test]
    fn phase_run_does_not_deadlock_on_large_stdout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec![
                helper_bin().to_str().unwrap().into(),
                "flood-stdout".into(),
                "262144".into(),
            ],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: Some(Duration::from_secs(10)),
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert_eq!(outcome.exit_code(), 0);
        assert!(
            outcome.stdout().len() > 64 * 1024,
            "expected >64KB stdout, got {}",
            outcome.stdout().len()
        );
    }

    #[test]
    fn phase_run_does_not_deadlock_on_large_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec![
                helper_bin().to_str().unwrap().into(),
                "flood-stderr".into(),
                "262144".into(),
            ],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: Some(Duration::from_secs(10)),
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(!outcome.timed_out());
        assert!(
            outcome.stderr().len() > 64 * 1024,
            "expected >64KB stderr, got {}",
            outcome.stderr().len()
        );
    }

    #[cfg(unix)]
    #[test]
    fn phase_run_timeout_kills_grandchild_processes() {
        let dir = tempfile::tempdir().unwrap();
        let pid_file = dir.path().join("grandchild.pid");
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec![
                helper_bin().to_str().unwrap().into(),
                "spawn-and-wait".into(),
                pid_file.to_str().unwrap().into(),
            ],
            pwd: crate::file::Twig::new(PathBuf::from(".")).unwrap(),
            timeout_absolute: Some(Duration::from_millis(500)),
            ..make_phase(&root)
        };
        let outcome = phase.run(None).unwrap();
        assert!(outcome.timed_out());

        std::thread::sleep(Duration::from_millis(100));

        let pid_str = std::fs::read_to_string(&pid_file).expect("grandchild pid file");
        let pid: i32 = pid_str.trim().parse().expect("parse pid");
        let alive = unsafe { libc::kill(pid, 0) } == 0;
        assert!(
            !alive,
            "grandchild process {pid} should have been killed but is still running"
        );
    }
}

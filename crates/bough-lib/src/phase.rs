#[cfg(test)]
use std::path::{Path, PathBuf};
use std::{collections::HashMap, time::Duration};

use bough_fs::{File, Root, Twig};
use tracing::{info, warn};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    EmptyCommand,
    File(bough_fs::Error),
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

impl From<bough_fs::Error> for Error {
    fn from(e: bough_fs::Error) -> Self {
        Error::File(e)
    }
}

const GRACEFUL_SHUTDOWN_PERIOD: Duration = Duration::from_secs(10);

fn kill_process_tree(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        use wait_timeout::ChildExt;
        let pid = child.id() as i32;

        unsafe {
            libc::kill(-pid, libc::SIGTERM);
        }

        match child.wait_timeout(GRACEFUL_SHUTDOWN_PERIOD) {
            Ok(Some(_)) => return,
            _ => {}
        }

        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
    }
}

pub struct Phase<'a, R: Root> {
    root: &'a R,
    pwd: Twig,
    env: HashMap<String, String>,
    cmd: Vec<String>,
    timeout: Option<Duration>,
}

impl<'a> Phase<'a, crate::base::Base> {
    pub fn new(
        root: &'a crate::base::Base,
        pwd: Twig,
        env: HashMap<String, String>,
        cmd: Vec<String>,
        timeout: Option<Duration>,
    ) -> Self {
        Self {
            root,
            pwd,
            env,
            cmd,
            timeout,
        }
    }
}

impl<'a> Phase<'a, crate::workspace::Workspace> {
    pub fn new(
        root: &'a crate::workspace::Workspace,
        pwd: Twig,
        env: HashMap<String, String>,
        cmd: Vec<String>,
        timeout: Duration,
    ) -> Self {
        Self {
            root,
            pwd,
            env,
            cmd,
            timeout: Some(timeout),
        }
    }
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

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn run(&self) -> Result<PhaseOutcome, Error> {
        use wait_timeout::ChildExt;

        if self.cmd.is_empty() {
            warn!("attempted to run phase with empty command");
            return Err(Error::EmptyCommand);
        }

        let working_dir = File::new(self.root, &self.pwd).resolve();

        info!(
            cmd = ?self.cmd,
            pwd = %working_dir.display(),
            timeout = ?self.timeout,
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

        let timed_out = if let Some(timeout) = self.timeout {
            match child.wait_timeout(timeout)? {
                Some(_status) => false,
                None => {
                    kill_process_tree(&mut child);
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

        if timed_out {
            info!(
                cmd = ?self.cmd,
                duration_ms = duration.as_millis() as u64,
                "phase timed out"
            );
            Ok(PhaseOutcome::TimedOut {
                stdout,
                stderr,
                duration,
            })
        } else {
            let exit_code = child.try_wait()?.and_then(|s| s.code()).unwrap_or(-1);
            info!(
                cmd = ?self.cmd,
                exit_code,
                duration_ms = duration.as_millis() as u64,
                "phase completed"
            );
            Ok(PhaseOutcome::Completed {
                stdout,
                stderr,
                exit_code,
                duration,
            })
        }
    }
}

#[derive(Debug)]
pub enum PhaseOutcome {
    Completed {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        exit_code: i32,
        duration: std::time::Duration,
    },
    TimedOut {
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        duration: std::time::Duration,
    },
}

impl PhaseOutcome {
    pub fn stdout(&self) -> &[u8] {
        match self {
            PhaseOutcome::Completed { stdout, .. } | PhaseOutcome::TimedOut { stdout, .. } => {
                stdout
            }
        }
    }

    pub fn stderr(&self) -> &[u8] {
        match self {
            PhaseOutcome::Completed { stderr, .. } | PhaseOutcome::TimedOut { stderr, .. } => {
                stderr
            }
        }
    }

    pub fn duration(&self) -> std::time::Duration {
        match self {
            PhaseOutcome::Completed { duration, .. } | PhaseOutcome::TimedOut { duration, .. } => {
                *duration
            }
        }
    }
}

fn parse_cmd_and_pwd(cmd: &str, pwd: std::path::PathBuf) -> Result<(Twig, Vec<String>), Error> {
    if pwd.is_absolute() {
        return Err(Error::AbsolutePwd(pwd));
    }
    let twig = Twig::new(pwd).map_err(bough_fs::Error::from)?;
    let cmd_parts: Vec<String> = cmd.split_whitespace().map(String::from).collect();
    Ok((twig, cmd_parts))
}

pub fn run_phase_in_base(
    root: &crate::base::Base,
    cmd: &str,
    pwd: std::path::PathBuf,
    env: HashMap<String, String>,
    timeout: Option<chrono::Duration>,
) -> Result<PhaseOutcome, Error> {
    let (twig, cmd_parts) = parse_cmd_and_pwd(cmd, pwd)?;
    let std_timeout = timeout
        .map(|d| d.to_std().map_err(|_| Error::InvalidTimeout))
        .transpose()?;
    let phase = Phase::<crate::base::Base>::new(root, twig, env, cmd_parts, std_timeout);
    phase.run()
}

pub fn run_phase_in_workspace(
    root: &crate::workspace::Workspace,
    cmd: &str,
    pwd: std::path::PathBuf,
    env: HashMap<String, String>,
    timeout: chrono::Duration,
) -> Result<PhaseOutcome, Error> {
    let (twig, cmd_parts) = parse_cmd_and_pwd(cmd, pwd)?;
    let std_timeout = timeout.to_std().map_err(|_| Error::InvalidTimeout)?;
    let phase = Phase::<crate::workspace::Workspace>::new(root, twig, env, cmd_parts, std_timeout);
    phase.run()
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
            pwd: bough_fs::Twig::new(PathBuf::from("src")).unwrap(),
            env: HashMap::new(),
            cmd: vec!["echo".into(), "hello".into()],
            timeout: None,
        }
    }

    #[test]
    fn phase_holds_root() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let phase = make_phase(&root);
        assert_eq!(phase.root().path(), Path::new("/tmp/project"));
    }

    #[test]
    fn phase_holds_pwd_twig() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let pwd = bough_fs::Twig::new(PathBuf::from("src/test")).unwrap();
        let phase = Phase {
            pwd,
            ..make_phase(&root)
        };
        assert_eq!(phase.pwd().path(), Path::new("src/test"));
    }

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

    #[test]
    fn phase_holds_timeout() {
        let root = TestRoot(PathBuf::from("/tmp/project"));
        let phase = Phase {
            timeout: Some(Duration::from_secs(30)),
            ..make_phase(&root)
        };
        assert_eq!(phase.timeout(), Some(Duration::from_secs(30)));
    }

    #[test]
    fn phase_outcome_completed_holds_all_fields() {
        let outcome = PhaseOutcome::Completed {
            stdout: b"hello\n".to_vec(),
            stderr: b"warn\n".to_vec(),
            exit_code: 0,
            duration: std::time::Duration::from_millis(150),
        };
        assert_eq!(outcome.stdout(), b"hello\n");
        assert_eq!(outcome.stderr(), b"warn\n");
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 0, .. }
        ));
        assert_eq!(outcome.duration(), std::time::Duration::from_millis(150));
    }

    #[test]
    fn phase_outcome_nonzero_exit_is_not_error() {
        let outcome = PhaseOutcome::Completed {
            stdout: vec![],
            stderr: b"error\n".to_vec(),
            exit_code: 1,
            duration: std::time::Duration::from_millis(50),
        };
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 1, .. }
        ));
    }

    #[test]
    fn phase_outcome_timed_out_holds_fields() {
        let outcome = PhaseOutcome::TimedOut {
            stdout: b"partial\n".to_vec(),
            stderr: vec![],
            duration: std::time::Duration::from_millis(100),
        };
        assert_eq!(outcome.stdout(), b"partial\n");
        assert_eq!(outcome.stderr(), b"");
        assert_eq!(outcome.duration(), std::time::Duration::from_millis(100));
        assert!(matches!(outcome, PhaseOutcome::TimedOut { .. }));
    }

    #[test]
    fn phase_run_executes_command() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "hello".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 0, .. }
        ));
        assert_eq!(String::from_utf8_lossy(outcome.stdout()).trim(), "hello");
    }

    #[test]
    fn phase_run_nonzero_exit_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "exit 42".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 42, .. }
        ));
    }

    #[test]
    fn phase_run_uses_pwd() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["pwd".into()],
            pwd: bough_fs::Twig::new(PathBuf::from("subdir")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        let out = String::from_utf8_lossy(outcome.stdout());
        assert!(
            out.trim().ends_with("subdir"),
            "pwd should end with subdir, got: {out}"
        );
    }

    #[test]
    fn phase_run_applies_env() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "echo $MY_VAR".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            env: HashMap::from([("MY_VAR".into(), "hello_env".into())]),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(
            String::from_utf8_lossy(outcome.stdout()).trim(),
            "hello_env"
        );
    }

    #[test]
    fn phase_run_captures_stderr() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sh".into(), "-c".into(), "echo err >&2".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert_eq!(String::from_utf8_lossy(outcome.stderr()).trim(), "err");
    }

    #[test]
    fn phase_run_records_duration() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "0.05".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(outcome.duration() >= std::time::Duration::from_millis(40));
    }

    #[test]
    fn phase_run_kills_on_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["sleep".into(), "10".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_millis(100)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(outcome, PhaseOutcome::TimedOut { .. }));
        assert!(outcome.duration() < std::time::Duration::from_secs(5));
    }

    #[test]
    fn phase_run_no_timeout_when_command_finishes_in_time() {
        let dir = tempfile::tempdir().unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec!["echo".into(), "fast".into()],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_millis(100)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 0, .. }
        ));
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
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_secs(10)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(
            outcome,
            PhaseOutcome::Completed { exit_code: 0, .. }
        ));
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
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_secs(10)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(outcome, PhaseOutcome::Completed { .. }));
        assert!(
            outcome.stderr().len() > 64 * 1024,
            "expected >64KB stderr, got {}",
            outcome.stderr().len()
        );
    }

    #[cfg(unix)]
    #[test]
    fn phase_run_timeout_kills_deep_process_tree() {
        let dir = tempfile::tempdir().unwrap();
        let pid_dir = dir.path().join("pids");
        std::fs::create_dir_all(&pid_dir).unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec![
                helper_bin().to_str().unwrap().into(),
                "spawn-chain".into(),
                pid_dir.to_str().unwrap().into(),
                "3".into(),
            ],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_millis(500)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(outcome, PhaseOutcome::TimedOut { .. }));

        std::thread::sleep(Duration::from_millis(200));

        for depth in 0..=3 {
            let pid_file = pid_dir.join(format!("depth-{depth}.pid"));
            let pid_str = std::fs::read_to_string(&pid_file)
                .unwrap_or_else(|_| panic!("missing pid file for depth {depth}"));
            let pid: i32 = pid_str.trim().parse().expect("parse pid");
            let alive = unsafe { libc::kill(pid, 0) } == 0;
            assert!(
                !alive,
                "process at depth {depth} (pid {pid}) should have been killed but is still running"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn phase_run_timeout_gracefully_terminates_nested_process_groups() {
        let dir = tempfile::tempdir().unwrap();
        let pid_dir = dir.path().join("pids");
        std::fs::create_dir_all(&pid_dir).unwrap();
        let root = TestRoot(dir.path().to_path_buf());
        let phase = Phase {
            cmd: vec![
                helper_bin().to_str().unwrap().into(),
                "spawn-own-pgroup".into(),
                pid_dir.to_str().unwrap().into(),
            ],
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_millis(500)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(outcome, PhaseOutcome::TimedOut { .. }));

        std::thread::sleep(Duration::from_millis(500));

        let parent_pid: i32 = std::fs::read_to_string(pid_dir.join("parent.pid"))
            .expect("parent pid file")
            .trim()
            .parse()
            .expect("parse parent pid");
        let child_pid: i32 = std::fs::read_to_string(pid_dir.join("child.pid"))
            .expect("child pid file")
            .trim()
            .parse()
            .expect("parse child pid");

        let parent_alive = unsafe { libc::kill(parent_pid, 0) } == 0;
        let child_alive = unsafe { libc::kill(child_pid, 0) } == 0;

        assert!(
            !parent_alive,
            "parent (pid {parent_pid}) should have been killed but is still running"
        );
        assert!(
            !child_alive,
            "child in own pgroup (pid {child_pid}) should have been killed but is still running"
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
            pwd: bough_fs::Twig::new(PathBuf::from(".")).unwrap(),
            timeout: Some(Duration::from_millis(500)),
            ..make_phase(&root)
        };
        let outcome = phase.run().unwrap();
        assert!(matches!(outcome, PhaseOutcome::TimedOut { .. }));

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

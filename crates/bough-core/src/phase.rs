use crate::config::TimeoutConfig;
use crate::WorkspaceId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Init,
    Reset,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunIn {
    SourceDir,
    Workspace(WorkspaceId),
}

#[derive(Debug)]
pub enum Error {
    Command {
        index: usize,
        cmd: String,
        source: std::io::Error,
    },
    Timeout {
        index: usize,
        cmd: String,
        stdout: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Command { index, cmd, source } => {
                write!(f, "command [{index}] `{cmd}` failed to start: {source}")
            }
            Error::Timeout { index, cmd, .. } => {
                write!(f, "command [{index}] `{cmd}` timed out")
            }
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Default)]
pub struct PhaseOutput {
    pub stdout: String,
    pub error_code: Option<i32>,
}

#[derive(Debug)]
pub struct PhaseRunner {
    pub pwd: PathBuf,
    pub env: HashMap<String, String>,
    pub timeout: TimeoutConfig,
    pub commands: String,
}

impl PhaseRunner {
    pub fn run(&self) -> Result<PhaseOutput, Error> {
        let timeout = self.timeout.absolute.map(Duration::from_secs);
        let (stdout, code) = self.run_one(0, &self.commands, &self.pwd, &timeout)?;
        Ok(PhaseOutput { stdout, error_code: if code != 0 { Some(code) } else { None } })
    }

    fn run_one(
        &self,
        index: usize,
        cmd: &str,
        pwd: &Path,
        timeout: &Option<Duration>,
    ) -> Result<(String, i32), Error> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(pwd)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| Error::Command { index, cmd: cmd.to_string(), source: e })?;

        let read_stdout = |child: &mut std::process::Child| -> String {
            let mut buf = String::new();
            if let Some(mut out) = child.stdout.take() {
                let _ = out.read_to_string(&mut buf);
            }
            buf
        };

        match timeout {
            Some(duration) => {
                let status = child
                    .wait_timeout(*duration)
                    .map_err(|e| Error::Command { index, cmd: cmd.to_string(), source: e })?;
                let out = read_stdout(&mut child);
                match status {
                    Some(exit) => Ok((out, exit.code().unwrap_or(-1))),
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                        Err(Error::Timeout { index, cmd: cmd.to_string(), stdout: out })
                    }
                }
            }
            None => {
                let out = read_stdout(&mut child);
                let status = child
                    .wait()
                    .map_err(|e| Error::Command { index, cmd: cmd.to_string(), source: e })?;
                Ok((out, status.code().unwrap_or(-1)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ConfigBuilder, ConfigError};
    use crate::WorkspaceId;
    use std::path::PathBuf;

    fn config(source_dir: &str, toml: &str) -> crate::config::Config {
        let tv: toml::Value = toml::from_str(toml).unwrap();
        ConfigBuilder::new(PathBuf::from(source_dir))
            .from_value(serde_value::to_value(tv).unwrap())
            .build()
            .unwrap()
    }

    #[test]
    fn source_dir_is_pwd_when_no_pwd_set() {
        let c = config("/src", r#"
            [test]
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src"));
    }

    #[test]
    fn phase_pwd_joined_to_source_dir() {
        let c = config("/src", r#"
            [test]
            pwd = "subdir"
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/subdir"));
    }

    #[test]
    fn meta_default_pwd_used_when_phase_has_none() {
        let c = config("/src", r#"
            pwd = "default-dir"
            [test]
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/default-dir"));
    }

    #[test]
    fn phase_pwd_takes_precedence_over_meta_default() {
        let c = config("/src", r#"
            pwd = "default-dir"
            [test]
            pwd = "phase-dir"
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/phase-dir"));
    }

    #[test]
    fn workspace_absolute_bough_dir() {
        let c = config("/src", r#"
            bough_dir = "/bough"
            [test]
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::Workspace(WorkspaceId::from_trusted("ws"))).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/bough/workspaces/ws"));
    }

    #[test]
    fn workspace_relative_bough_dir_resolved_from_source_dir() {
        let c = config("/src", r#"
            bough_dir = ".bough"
            [test]
            commands = "run"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::Workspace(WorkspaceId::from_trusted("ws"))).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/.bough/workspaces/ws"));
    }

    #[test]
    fn env_merge_phase_overrides_meta_default() {
        let c = config("/src", r#"
            [env]
            SHARED = "default"
            DEFAULT_ONLY = "d"
            [test]
            commands = "run"
            [test.env]
            SHARED = "phase"
            PHASE_ONLY = "p"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.env["SHARED"], "phase");
        assert_eq!(r.env["DEFAULT_ONLY"], "d");
        assert_eq!(r.env["PHASE_ONLY"], "p");
    }

    #[test]
    fn timeout_phase_overrides_meta_default_per_field() {
        let c = config("/src", r#"
            timeout.absolute = 60
            timeout.relative = 5
            [test]
            commands = "run"
            [test.timeout]
            absolute = 30
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.timeout.absolute, Some(30));
        assert_eq!(r.timeout.relative, Some(5));
    }

    #[test]
    fn commands_taken_from_phase() {
        let c = config("/src", r#"
            [test]
            commands = "step1"
        "#);
        let r = c.new_phase_runner(Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.commands, "step1");
    }

    #[test]
    fn phase_not_configured_returns_error() {
        let c = config("/src", r#"
            [test]
            commands = "run"
        "#);
        let err = c.new_phase_runner(Phase::Init, RunIn::SourceDir).unwrap_err();
        assert!(matches!(err, ConfigError::PhaseNotConfigured { .. }));
    }
}

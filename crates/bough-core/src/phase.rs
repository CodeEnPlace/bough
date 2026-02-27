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
    pub commands: Vec<String>,
}

impl PhaseRunner {
    pub fn run(&self) -> Result<PhaseOutput, Error> {
        let timeout = self.timeout.absolute.map(Duration::from_secs);
        let mut stdout = String::new();
        for (i, cmd) in self.commands.iter().enumerate() {
            let (chunk, code) = self.run_one(i, cmd, &self.pwd, &timeout)?;
            stdout.push_str(&chunk);
            if code != 0 {
                return Ok(PhaseOutput { stdout, error_code: Some(code) });
            }
        }
        Ok(PhaseOutput { stdout, error_code: None })
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
    use crate::config::{ConfigBuilder, ConfigError, SuiteId};
    use crate::WorkspaceId;
    use std::path::PathBuf;

    fn config(source_dir: &str, toml: &str) -> crate::config::Config {
        let tv: toml::Value = toml::from_str(toml).unwrap();
        ConfigBuilder::new(PathBuf::from(source_dir))
            .from_value(serde_value::to_value(tv).unwrap())
            .build()
            .unwrap()
    }

    fn sid(config: &crate::config::Config, name: &str) -> SuiteId {
        SuiteId::try_parse(config, name).unwrap()
    }

    #[test]
    fn source_dir_is_pwd_when_no_pwd_set() {
        let c = config("/src", r#"
            [suite.test]
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src"));
    }

    #[test]
    fn phase_pwd_joined_to_source_dir() {
        let c = config("/src", r#"
            [suite.test]
            pwd = "subdir"
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/subdir"));
    }

    #[test]
    fn suite_default_pwd_used_when_phase_has_none() {
        let c = config("/src", r#"
            [suite]
            pwd = "suite-dir"
            [suite.test]
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/suite-dir"));
    }

    #[test]
    fn phase_pwd_takes_precedence_over_suite_default() {
        let c = config("/src", r#"
            [suite]
            pwd = "suite-dir"
            [suite.test]
            pwd = "phase-dir"
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/phase-dir"));
    }

    #[test]
    fn workspace_absolute_bough_dir() {
        let c = config("/src", r#"
            bough_dir = "/bough"
            [suite.test]
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::Workspace(WorkspaceId::from_trusted("ws"))).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/bough/workspaces/ws"));
    }

    #[test]
    fn workspace_relative_bough_dir_resolved_from_source_dir() {
        let c = config("/src", r#"
            bough_dir = ".bough"
            [suite.test]
            commands = ["run"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::Workspace(WorkspaceId::from_trusted("ws"))).unwrap();
        assert_eq!(r.pwd, PathBuf::from("/src/.bough/workspaces/ws"));
    }

    #[test]
    fn env_merge_phase_overrides_suite() {
        let c = config("/src", r#"
            [suite.env]
            SHARED = "suite"
            SUITE_ONLY = "s"
            [suite.test]
            commands = ["run"]
            [suite.test.env]
            SHARED = "phase"
            PHASE_ONLY = "p"
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.env["SHARED"], "phase");
        assert_eq!(r.env["SUITE_ONLY"], "s");
        assert_eq!(r.env["PHASE_ONLY"], "p");
    }

    #[test]
    fn timeout_phase_overrides_suite_per_field() {
        let c = config("/src", r#"
            [suite.timeout]
            absolute = 60
            relative = 5
            [suite.test]
            commands = ["run"]
            [suite.test.timeout]
            absolute = 30
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.timeout.absolute, Some(30));
        assert_eq!(r.timeout.relative, Some(5));
    }

    #[test]
    fn commands_taken_from_phase() {
        let c = config("/src", r#"
            [suite.test]
            commands = ["step1", "step2"]
        "#);
        let r = c.new_phase_runner(&sid(&c, "suite"), Phase::Test, RunIn::SourceDir).unwrap();
        assert_eq!(r.commands, vec!["step1", "step2"]);
    }

    #[test]
    fn unknown_suite_returns_error() {
        let c = config("/src", "");
        let err = c.new_phase_runner(&SuiteId("missing".into()), Phase::Test, RunIn::SourceDir).unwrap_err();
        assert!(matches!(err, ConfigError::UnknownSuite { .. }));
    }

    #[test]
    fn phase_not_configured_returns_error() {
        let c = config("/src", r#"
            [suite.test]
            commands = ["run"]
        "#);
        let err = c.new_phase_runner(&sid(&c, "suite"), Phase::Init, RunIn::SourceDir).unwrap_err();
        assert!(matches!(err, ConfigError::PhaseNotConfigured { .. }));
    }
}

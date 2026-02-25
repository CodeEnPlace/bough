use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Debug)]
pub enum Error {
    Command {
        index: usize,
        cmd: String,
        source: std::io::Error,
    },
    CommandFailed {
        index: usize,
        cmd: String,
        code: i32,
        #[allow(dead_code)]
        stdout: String,
    },
    Timeout {
        index: usize,
        cmd: String,
        #[allow(dead_code)]
        stdout: String,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Command { index, cmd, source } => {
                write!(f, "command [{index}] `{cmd}` failed to start: {source}")
            }
            Error::CommandFailed { index, cmd, code, .. } => {
                write!(f, "command [{index}] `{cmd}` exited with code {code}")
            }
            Error::Timeout { index, cmd, .. } => {
                write!(f, "command [{index}] `{cmd}` timed out")
            }
        }
    }
}

impl std::error::Error for Error {}

pub struct PhaseRunner<'a> {
    commands: Vec<String>,
    pwd: &'a Path,
    env: HashMap<String, String>,
    timeout: Option<Duration>,
}

#[derive(Debug, Default)]
pub struct PhaseOutput {
    pub stdout: String,
}

impl<'a> PhaseRunner<'a> {
    pub fn new(
        commands: Vec<String>,
        pwd: &'a Path,
        env: HashMap<String, String>,
        timeout_secs: Option<u64>,
    ) -> Self {
        Self {
            commands,
            pwd,
            env,
            timeout: timeout_secs.map(Duration::from_secs),
        }
    }

    pub fn run(&self) -> Result<PhaseOutput, Error> {
        let mut combined_stdout = String::new();
        for (i, cmd) in self.commands.iter().enumerate() {
            let stdout = self.run_one(i, cmd)?;
            combined_stdout.push_str(&stdout);
        }
        Ok(PhaseOutput {
            stdout: combined_stdout,
        })
    }

    fn run_one(&self, index: usize, cmd: &str) -> Result<String, Error> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(&self.pwd)
            .envs(&self.env)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| Error::Command {
                index,
                cmd: cmd.to_string(),
                source: e,
            })?;

        let read_stdout = |child: &mut std::process::Child| -> String {
            let mut buf = String::new();
            if let Some(mut stdout) = child.stdout.take() {
                let _ = stdout.read_to_string(&mut buf);
            }
            buf
        };

        match self.timeout {
            Some(duration) => {
                let status = child
                    .wait_timeout(duration)
                    .map_err(|e| Error::Command {
                        index,
                        cmd: cmd.to_string(),
                        source: e,
                    })?;
                let stdout = read_stdout(&mut child);
                match status {
                    Some(exit) if exit.success() => Ok(stdout),
                    Some(exit) => Err(Error::CommandFailed {
                        index,
                        cmd: cmd.to_string(),
                        code: exit.code().unwrap_or(-1),
                        stdout,
                    }),
                    None => {
                        let _ = child.kill();
                        let _ = child.wait();
                        Err(Error::Timeout {
                            index,
                            cmd: cmd.to_string(),
                            stdout,
                        })
                    }
                }
            }
            None => {
                let stdout = read_stdout(&mut child);
                let status = child.wait().map_err(|e| Error::Command {
                    index,
                    cmd: cmd.to_string(),
                    source: e,
                })?;
                if status.success() {
                    Ok(stdout)
                } else {
                    Err(Error::CommandFailed {
                        index,
                        cmd: cmd.to_string(),
                        code: status.code().unwrap_or(-1),
                        stdout,
                    })
                }
            }
        }
    }
}

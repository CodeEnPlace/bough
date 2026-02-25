use bough_core::config::Config;
use serde::Serialize;
use std::process::Command;

use crate::render::{Render, color};

#[derive(Debug)]
pub enum Error {
    NoActiveRunner,
    NoTestIds,
    Run(std::io::Error),
    NonZero(i32),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NoActiveRunner => write!(f, "no active runner configured"),
            Error::NoTestIds => write!(f, "no test_ids.get_all configured for runner"),
            Error::Run(e) => write!(f, "failed to run get_all command: {e}"),
            Error::NonZero(code) => write!(f, "get_all command exited with code {code}"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Serialize)]
pub struct GetAllTestIds {
    pub test_ids: Vec<String>,
}

pub fn run(config: &Config) -> Result<GetAllTestIds, Error> {
    let runner_name = config.resolved_runner_name().ok_or(Error::NoActiveRunner)?;
    let cmd_str = config
        .runner_test_ids_get_all(runner_name)
        .ok_or(Error::NoTestIds)?;
    let pwd = config.runner_pwd(runner_name).ok_or(Error::NoActiveRunner)?;

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd_str)
        .current_dir(pwd)
        .output()
        .map_err(Error::Run)?;

    if !output.status.success() {
        return Err(Error::NonZero(output.status.code().unwrap_or(-1)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let test_ids = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    Ok(GetAllTestIds { test_ids })
}

impl Render for GetAllTestIds {
    fn render_value(&self) -> serde_value::Value {
        serde_value::to_value(&self.test_ids).expect("failed to serialize")
    }

    fn render_terse(&self) -> String {
        format!(
            "found {} test ids\n",
            color("\x1b[1m", &self.test_ids.len().to_string()),
        )
    }

    fn render_verbose(&self) -> String {
        let mut out = String::new();
        out.push_str(&color(
            "\x1b[1m",
            &format!("{} test ids", self.test_ids.len()),
        ));
        out.push('\n');
        for id in &self.test_ids {
            out.push_str(&format!("  {}\n", id));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let mut out = format!("{heading} Test IDs ({} entries)\n\n", self.test_ids.len());
        for id in &self.test_ids {
            out.push_str(&format!("- `{id}`\n"));
        }
        out.push('\n');
        out
    }
}

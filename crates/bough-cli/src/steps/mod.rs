pub mod apply_mutant_to_workspace;
pub mod cleanup;
pub mod create_workspaces;
pub mod derive_mutants;
pub mod find_files;
pub mod reset_workspace;
pub mod setup_workspace;
pub mod test_workspace;

use crate::io::{Render, Report, hashed_path};
use bough_core::plan::{Plan, WorkspaceManifest};
use bough_session::Session;
use serde::Serialize;
use std::path::PathBuf;

pub fn expand_glob(pattern: &str) -> Vec<PathBuf> {
    glob::glob(pattern)
        .unwrap_or_else(|e| {
            eprintln!("invalid glob pattern: {e}");
            std::process::exit(1);
        })
        .filter_map(|entry| match entry {
            Ok(path) if path.is_file() => Some(path),
            Ok(_) => None,
            Err(e) => {
                eprintln!("glob error: {e}");
                None
            }
        })
        .collect()
}

pub fn content_id(content: &str) -> String {
    bough_core::Hash::of(content).to_string()
}

pub fn read_plan(session: &Session) -> Plan {
    let pattern = session.directories.working.join("*.mutants.plan.json");
    let paths = expand_glob(&pattern.display().to_string());
    let plan_path = paths.first().unwrap_or_else(|| {
        eprintln!(
            "no plan file found in {}",
            session.directories.working.display()
        );
        std::process::exit(1);
    });
    let content = std::fs::read_to_string(plan_path).unwrap_or_else(|e| {
        eprintln!("failed to read plan: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("failed to parse plan: {e}");
        std::process::exit(1);
    })
}

pub fn read_workspace_manifest(session: &Session) -> WorkspaceManifest {
    let pattern = session.directories.working.join("*.workspaces.json");
    let paths = expand_glob(&pattern.display().to_string());
    let manifest_path = paths.first().unwrap_or_else(|| {
        eprintln!(
            "no workspace manifest found in {}",
            session.directories.working.display()
        );
        std::process::exit(1);
    });
    let content = std::fs::read_to_string(manifest_path).unwrap_or_else(|e| {
        eprintln!("failed to read workspace manifest: {e}");
        std::process::exit(1);
    });
    serde_json::from_str(&content).unwrap_or_else(|e| {
        eprintln!("failed to parse workspace manifest: {e}");
        std::process::exit(1);
    })
}

fn find_workspace<'a>(
    manifest: &'a WorkspaceManifest,
    name: &str,
) -> &'a bough_core::plan::Workspace {
    manifest
        .workspaces
        .iter()
        .find(|ws| ws.name == name)
        .unwrap_or_else(|| {
            eprintln!("workspace {name} not found in manifest");
            std::process::exit(1);
        })
}

fn run_in_workspace(
    session: &Session,
    workspace_name: &str,
    command: &Option<String>,
    step_name: &str,
) -> Option<CommandReport> {
    let cmd = match command {
        Some(c) => c,
        None => return None,
    };

    let manifest = read_workspace_manifest(session);
    let ws = find_workspace(&manifest, workspace_name);

    let output = std::process::Command::new("sh")
        .args(["-c", cmd])
        .current_dir(ws.path.join(&session.directories.sub))
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run {step_name}: {e}");
            std::process::exit(1);
        });

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let result = if output.status.success() {
        Ok(stdout)
    } else {
        Err(stderr)
    };

    Some(CommandReport {
        step: step_name.to_string(),
        workspace: workspace_name.to_string(),
        command: cmd.to_string(),
        result,
    })
}

use crate::io::color;

#[derive(Serialize)]
pub struct CommandReport {
    pub step: String,
    pub workspace: String,
    pub command: String,
    pub result: Result<String, String>,
}

impl Render for CommandReport {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        let (status, output) = match &self.result {
            Ok(stdout) => (color("\x1b[32m", "OK"), stdout.as_str()),
            Err(stderr) => (color("\x1b[31m", "FAIL"), stderr.as_str()),
        };
        let mut out = format!(
            "{} {} in {} [{}]\n",
            status, self.step, self.workspace, self.command
        );
        if !output.is_empty() {
            out.push_str(output);
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

impl Report for CommandReport {
    fn get_dir(&self, session: &bough_session::Session) -> PathBuf {
        session.directories.report.join("step").join(&self.step)
    }

    fn make_path(&self, session: &bough_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "command")
    }
}

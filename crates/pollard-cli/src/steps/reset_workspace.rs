use crate::io::{Action, Render, Report, Style, hashed_path};
use pollard_session::Session;
use crate::io::color;
use crate::steps::{find_workspace, read_workspace_manifest};
use pollard_core::config::Vcs;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(
    session: &Session,
    workspace_name: &str,
    rev: &str,
) -> (Vec<Action>, ResetWorkspaceReport) {
    match session.vcs {
        Vcs::Jj => {}
        other => {
            eprintln!("reset-workspace not yet implemented for {other:?}");
            std::process::exit(1);
        }
    }

    let manifest = read_workspace_manifest(session);
    let ws = find_workspace(&manifest, workspace_name);

    let output = std::process::Command::new("jj")
        .args(["edit", rev])
        .current_dir(&ws.path)
        .output()
        .unwrap_or_else(|e| {
            eprintln!("failed to run jj edit: {e}");
            std::process::exit(1);
        });

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("jj edit failed for workspace {}: {stderr}", ws.name);
        std::process::exit(1);
    }

    let report = ResetWorkspaceReport {
        workspace: workspace_name.to_string(),
        rev: rev.to_string(),
    };

    (vec![], report)
}

#[derive(Serialize)]
pub struct ResetWorkspaceReport {
    pub workspace: String,
    pub rev: String,
}

impl Render for ResetWorkspaceReport {
    fn render(&self, style: &Style, no_color: bool, _depth: u8) {
        let no_color = no_color || matches!(style, Style::Plain);
        match style {
            Style::Json => {
                println!(
                    "{}",
                    serde_json::to_string(self).expect("failed to serialize")
                );
            }
            Style::Plain | Style::Pretty | Style::Markdown => {
                println!(
                    "Will reset {} to {}",
                    color("\x1b[33m", &self.workspace, no_color),
                    color("\x1b[36m", &self.rev, no_color),
                );
            }
        }
    }
}

impl Report for ResetWorkspaceReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("step").join("reset-workspace")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "reset")
    }

}

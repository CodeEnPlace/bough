use crate::io::{Action, Report, Style};
use crate::session::Session;
use crate::steps::{color, find_workspace, read_workspace_manifest};
use pollard_core::config::Vcs;
use serde::Serialize;

pub fn run(session: &Session, workspace_name: &str, rev: &str) -> (Vec<Action>, ResetReport) {
    match session.vcs {
        Vcs::Jj => {}
        other => {
            eprintln!("step reset not yet implemented for {other:?}");
            std::process::exit(1);
        }
    }

    let manifest = read_workspace_manifest(session);
    let ws = find_workspace(&manifest, workspace_name);

    log::info!("resetting workspace {} to {rev}", ws.name);
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

    let report = ResetReport {
        workspace: workspace_name.to_string(),
        rev: rev.to_string(),
    };

    (vec![], report)
}

#[derive(Serialize)]
pub struct ResetReport {
    pub workspace: String,
    pub rev: String,
}

impl Report for ResetReport {
    fn render(&self, style: &Style, no_color: bool, _depth: u8) {
        match style {
            Style::Json => {
                println!(
                    "{}",
                    serde_json::to_string(self).expect("failed to serialize")
                );
            }
            Style::Pretty => {
                println!(
                    "Reset {} to {}",
                    color("\x1b[33m", &self.workspace, no_color),
                    color("\x1b[36m", &self.rev, no_color),
                );
            }
            Style::Plain | Style::Markdown => {
                println!("Reset {} to {}", self.workspace, self.rev);
            }
        }
    }
}

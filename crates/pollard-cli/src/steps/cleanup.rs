use crate::io::{Action, Render, Report, Style, hashed_path};
use pollard_session::Session;
use crate::steps::{color, expand_glob};
use serde::Serialize;
use std::path::PathBuf;

pub fn run(session: &Session) -> (Vec<Action>, CleanupReport) {
    let pattern = session.working_dir.join("*.workspaces.json");
    let manifest_paths = expand_glob(&pattern.display().to_string());

    let mut actions: Vec<Action> = Vec::new();
    let mut workspace_count = 0;

    for manifest_path in &manifest_paths {
        let content = std::fs::read_to_string(manifest_path).unwrap_or_else(|e| {
            eprintln!("failed to read {}: {e}", manifest_path.display());
            std::process::exit(1);
        });
        let manifest: pollard_core::plan::WorkspaceManifest = serde_json::from_str(&content)
            .unwrap_or_else(|e| {
                eprintln!("failed to parse {}: {e}", manifest_path.display());
                std::process::exit(1);
            });

        for ws in &manifest.workspaces {
            actions.push(Action::ForgetJjWorkspace {
                name: ws.name.clone(),
            });
            actions.push(Action::RemoveDir {
                path: ws.path.clone(),
            });
            workspace_count += 1;
        }

        actions.push(Action::RemoveFile {
            path: manifest_path.clone(),
        });
    }

    let report = CleanupReport {
        workspace_count,
        manifest_count: manifest_paths.len(),
    };

    (actions, report)
}

#[derive(Serialize)]
pub struct CleanupReport {
    pub workspace_count: usize,
    pub manifest_count: usize,
}

impl Render for CleanupReport {
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
                    "Will clean up {} workspaces from {} manifests",
                    color("\x1b[33m", &self.workspace_count.to_string(), no_color),
                    color("\x1b[36m", &self.manifest_count.to_string(), no_color),
                );
            }
            Style::Plain | Style::Markdown => {
                println!(
                    "Will clean up {} workspaces from {} manifests",
                    self.workspace_count, self.manifest_count,
                );
            }
        }
    }
}

impl Report for CleanupReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("step").join("cleanup")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "cleanup")
    }

}

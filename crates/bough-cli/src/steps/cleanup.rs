use crate::io::{Action, Render, Report, color, hashed_path};
use bough_session::Session;
use crate::steps::expand_glob;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(session: &Session) -> (Vec<Action>, CleanupReport) {
    let pattern = session.directories.working.join("*.workspaces.json");
    let manifest_paths = expand_glob(&pattern.display().to_string());

    let mut actions: Vec<Action> = Vec::new();
    let mut workspace_count = 0;

    for manifest_path in &manifest_paths {
        let content = std::fs::read_to_string(manifest_path).unwrap_or_else(|e| {
            eprintln!("failed to read {}: {e}", manifest_path.display());
            std::process::exit(1);
        });
        let manifest: bough_core::plan::WorkspaceManifest = serde_json::from_str(&content)
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
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        format!(
            "Will clean up {} workspaces from {} manifests\n",
            color("\x1b[33m", &self.workspace_count.to_string()),
            color("\x1b[36m", &self.manifest_count.to_string()),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

impl Report for CleanupReport {
    fn get_dir(&self, session: &bough_session::Session) -> PathBuf {
        session.directories.report.join("step").join("cleanup")
    }

    fn make_path(&self, session: &bough_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "cleanup")
    }

}

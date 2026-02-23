use crate::io::{Action, Render, Report, color, hashed_path};
use bough_session::Session;
use crate::steps::content_id;
use bough_core::config::Vcs;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(session: &Session) -> (Vec<Action>, CreateWorkspacesReport) {
    match &session.vcs {
        Vcs::Jj { .. } => {}
        other => {
            eprintln!("create-workspaces not yet implemented for {other:?}");
            std::process::exit(1);
        }
    }

    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let batch_id = &content_id(&nanos.to_string())[..8];

    let workspaces: Vec<bough_core::plan::Workspace> = (0..session.parallelism)
        .map(|i| {
            let name = format!("bough-{batch_id}-{i}");
            let path = session.directories.working.join(&name);
            bough_core::plan::Workspace { name, path }
        })
        .collect();

    let manifest = bough_core::plan::WorkspaceManifest {
        workspaces: workspaces.clone(),
    };
    let manifest_content =
        serde_json::to_string_pretty(&manifest).expect("failed to serialize manifest");
    let manifest_path = session
        .directories.working
        .join(format!("{}.workspaces.json", content_id(&manifest_content)));

    let mut actions = vec![Action::WriteFile {
        path: manifest_path.clone(),
        content: manifest_content,
    }];

    for ws in &workspaces {
        actions.push(Action::CreateJjWorkspace {
            name: ws.name.clone(),
            path: ws.path.clone(),
        });
    }

    let report = CreateWorkspacesReport {
        workspaces: workspaces.iter().map(|ws| ws.name.clone()).collect(),
        manifest: manifest_path,
    };

    (actions, report)
}

#[derive(Serialize)]
pub struct CreateWorkspacesReport {
    pub workspaces: Vec<String>,
    pub manifest: PathBuf,
}

impl Render for CreateWorkspacesReport {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        let mut out = format!(
            "Will create {} workspaces, manifest: {}\n",
            color("\x1b[33m", &self.workspaces.len().to_string()),
            color("\x1b[36m", &self.manifest.display().to_string()),
        );
        for ws in &self.workspaces {
            out.push_str(&format!("  {}\n", color("\x1b[33m", ws)));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

impl Report for CreateWorkspacesReport {
    fn get_dir(&self, session: &bough_session::Session) -> PathBuf {
        session.directories.report.join("step").join("create-workspaces")
    }

    fn make_path(&self, session: &bough_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "workspaces")
    }

}

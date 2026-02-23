use crate::io::{Action, Render, Report, Style, hashed_path};
use pollard_session::Session;
use crate::io::color;
use crate::steps::content_id;
use pollard_core::config::Vcs;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(session: &Session) -> (Vec<Action>, CreateWorkspacesReport) {
    match session.vcs {
        Vcs::Jj => {}
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

    let workspaces: Vec<pollard_core::plan::Workspace> = (0..session.parallelism)
        .map(|i| {
            let name = format!("pollard-{batch_id}-{i}");
            let path = session.working_dir.join(&name);
            pollard_core::plan::Workspace { name, path }
        })
        .collect();

    let manifest = pollard_core::plan::WorkspaceManifest {
        workspaces: workspaces.clone(),
    };
    let manifest_content =
        serde_json::to_string_pretty(&manifest).expect("failed to serialize manifest");
    let manifest_path = session
        .working_dir
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
                    "Will create {} workspaces, manifest: {}",
                    color("\x1b[33m", &self.workspaces.len().to_string(), no_color),
                    color("\x1b[36m", &self.manifest.display().to_string(), no_color),
                );
                for ws in &self.workspaces {
                    println!("  {}", color("\x1b[33m", ws, no_color));
                }
            }
        }
    }
}

impl Report for CreateWorkspacesReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("step").join("create-workspaces")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "workspaces")
    }

}

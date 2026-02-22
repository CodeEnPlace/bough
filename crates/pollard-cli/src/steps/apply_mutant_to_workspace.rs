use crate::io::{Action, Report, Style, hashed_path};
use crate::session::Session;
use crate::steps::{color, find_workspace, read_plan, read_workspace_manifest};
use pollard_core::Hash;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(
    session: &Session,
    workspace_name: &str,
    hash: &Hash,
) -> (Vec<Action>, ApplyMutantToWorkspaceReport) {
    let plan = read_plan(session);
    let manifest = read_workspace_manifest(session);
    let ws = find_workspace(&manifest, workspace_name);

    let entry = plan
        .entries
        .iter()
        .find(|e| &e.mutated_hash == hash)
        .unwrap_or_else(|| {
            eprintln!("mutation {hash} not found in plan");
            std::process::exit(1);
        });

    let file_in_workspace = ws.path.join(&entry.source_path);
    let source_content = std::fs::read_to_string(&file_in_workspace).unwrap_or_else(|e| {
        eprintln!("failed to read {}: {e}", file_in_workspace.display());
        std::process::exit(1);
    });

    let source_hash = pollard_core::Hash::of(&source_content);
    if source_hash != entry.source_hash {
        eprintln!(
            "source hash mismatch for {}: expected {}, got {}",
            entry.source_path.display(),
            entry.source_hash,
            source_hash,
        );
        std::process::exit(1);
    }

    let mut mutated = String::with_capacity(source_content.len());
    mutated.push_str(&source_content[..entry.start_byte]);
    mutated.push_str(&entry.replacement);
    mutated.push_str(&source_content[entry.end_byte..]);

    std::fs::write(&file_in_workspace, &mutated).unwrap_or_else(|e| {
        eprintln!("failed to write {}: {e}", file_in_workspace.display());
        std::process::exit(1);
    });

    let report = ApplyMutantToWorkspaceReport {
        source_path: file_in_workspace,
        source_hash: entry.source_hash.clone(),
        mutated_hash: entry.mutated_hash.clone(),
    };

    (vec![], report)
}

#[derive(Serialize)]
pub struct ApplyMutantToWorkspaceReport {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
}

impl Report for ApplyMutantToWorkspaceReport {
    fn get_dir(&self, session: &crate::session::Session) -> PathBuf {
        session
            .report_dir
            .join("step")
            .join("apply-mutant-to-workspace")
    }

    fn make_path(&self, session: &crate::session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "applied")
    }

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
                    "Applied mutation {} to {}",
                    color("\x1b[33m", &self.mutated_hash.to_string(), no_color),
                    color(
                        "\x1b[36m",
                        &self.source_path.display().to_string(),
                        no_color
                    ),
                );
            }
            Style::Plain | Style::Markdown => {
                println!(
                    "Applied mutation {} to {}",
                    self.mutated_hash,
                    self.source_path.display(),
                );
            }
        }
    }
}

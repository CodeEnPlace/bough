use crate::io::{Action, Render, color};
use crate::mutate::find_mutated;
use bough_core::config::LanguageId;
use bough_core::{Hash, SourceFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn run(
    language: &LanguageId,
    input: &Path,
    hash: &Hash,
) -> (Vec<Action>, ApplyReport) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = find_mutated(language, &file, hash);

    let actions = vec![Action::WriteFile {
        path: input.to_owned(),
        content: mutated.content().to_string(),
    }];

    let report = ApplyReport {
        source_path: file.path().to_owned(),
        source_hash: file.hash().clone(),
        mutated_hash: mutated.hash().clone(),
    };

    (actions, report)
}

#[derive(Serialize)]
pub struct ApplyReport {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
}

impl Render for ApplyReport {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        format!(
            "Will apply mutation {} to {}\n",
            color("\x1b[33m", &self.mutated_hash.to_string()),
            color("\x1b[36m", &self.source_path.display().to_string()),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

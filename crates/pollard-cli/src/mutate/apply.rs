use crate::io::{Action, Render, Report, Style, hashed_path};
use crate::mutate::find_mutated;
use pollard_core::config::LanguageId;
use pollard_core::{Hash, SourceFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::io::color;

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
                    "Will apply mutation {} to {}",
                    color("\x1b[33m", &self.mutated_hash.to_string(), no_color),
                    color(
                        "\x1b[36m",
                        &self.source_path.display().to_string(),
                        no_color
                    ),
                );
            }
        }
    }
}

impl Report for ApplyReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("mutate").join("apply")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "applied")
    }

}

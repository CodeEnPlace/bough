use crate::io::{Action, Render, Report, color, hashed_path};
use crate::mutate::find_description;
use bough_core::config::LanguageId;
use bough_core::{Hash, MutationKind, SourceFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn run(language: &LanguageId, input: &Path, hash: &Hash) -> (Vec<Action>, DescribeReport) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let desc = find_description(language, &file, hash);

    let report = DescribeReport {
        path: desc.path,
        kind: desc.kind,
        code_tag: desc.code_tag,
        start_line: desc.start_line,
        start_char: desc.start_char,
        end_line: desc.end_line,
        end_char: desc.end_char,
        original: desc.original,
        replacement: desc.replacement,
        mutated_hash: desc.mutated_hash,
    };

    (vec![], report)
}

#[derive(Serialize)]
pub struct DescribeReport {
    pub path: PathBuf,
    pub kind: MutationKind,
    pub code_tag: &'static str,
    pub mutated_hash: Hash,
    pub start_line: usize,
    pub start_char: usize,
    pub end_line: usize,
    pub end_char: usize,
    pub original: String,
    pub replacement: String,
}

impl DescribeReport {
    fn location(&self) -> String {
        format!(
            "{}:{}-{}:{}",
            self.start_line + 1,
            self.start_char + 1,
            self.end_line + 1,
            self.end_char + 1,
        )
    }
}

impl Render for DescribeReport {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, depth: u8) -> String {
        let path = self.path.display();
        let loc = self.location();
        format!(
            "{} at {}\n{}\n{}\n",
            color("\x1b[1m", &self.kind.render_pretty(depth + 1)),
            color("\x1b[36m", &format!("{path}:{loc}")),
            color("\x1b[31m", &self.original),
            color("\x1b[32m", &self.replacement),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let path = self.path.display();
        let loc = self.location();
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let tag = self.code_tag;
        format!(
            "{heading} Mutation\n\n\
             **Kind:** {}\n\n\
             **File:** `{path}`\n\n\
             **Location:** {loc}\n\n\
             **Original:**\n```{tag}\n{}\n```\n\n\
             **Replacement:**\n```{tag}\n{}\n```\n",
            self.kind.render_markdown(depth + 1),
            self.original,
            self.replacement,
        )
    }
}

impl Report for DescribeReport {
    fn get_dir(&self, session: &bough_session::Session) -> PathBuf {
        session.directories.report.join("mutate").join("describe")
    }

    fn make_path(&self, session: &bough_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "describe")
    }
}

use crate::io::{Action, Render, Report, Style, hashed_path};
use crate::mutate::find_description;
use pollard_core::config::LanguageId;
use pollard_core::{Hash, MutationKind, SourceFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn run(language: &LanguageId, input: &Path, hash: &Hash) -> (Vec<Action>, DescribeReport) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let desc = find_description(language, &file, hash);

    let report = DescribeReport {
        path: desc.path,
        kind: desc.kind,
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
    fn render(&self, style: &Style, no_color: bool, depth: u8) {
        let path = self.path.display();
        let loc = self.location();

        match style {
            Style::Json => {
                println!(
                    "{}",
                    serde_json::to_string(self).expect("failed to serialize")
                );
            }
            Style::Markdown => {
                let heading = "#".repeat((depth + 1).min(6) as usize);
                println!("{heading} Mutation\n");
                print!("**Kind:** ");
                self.kind.render(style, no_color, depth + 1);
                println!("\n");
                println!("**File:** `{path}`\n");
                println!("**Location:** {loc}\n");
                println!("**Original:**\n```\n{}\n```\n", self.original);
                println!("**Replacement:**\n```\n{}\n```", self.replacement);
            }
            Style::Pretty => {
                print!("\x1b[1m");
                self.kind.render(style, no_color, depth + 1);
                println!("\x1b[0m at \x1b[36m{path}:{loc}\x1b[0m");
                println!("\x1b[31m{}\x1b[0m", self.original);
                println!("\x1b[32m{}\x1b[0m", self.replacement);
            }
            Style::Plain => {
                self.kind.render(style, no_color, depth + 1);
                println!(" at {path}:{loc}");
                println!("{}", self.original);
                println!("{}", self.replacement);
            }
        }
    }
}

impl Report for DescribeReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("mutate").join("describe")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "describe")
    }
}

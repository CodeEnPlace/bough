use crate::io::{Action, Render, Report, Style, hashed_path};
use crate::mutate::find_description;
use pollard_core::config::LanguageId;
use pollard_core::{Hash, MutationKind, SourceFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::io::color;

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
                let tag = self.code_tag;
                println!("**Original:**\n```{tag}\n{}\n```\n", self.original);
                println!("**Replacement:**\n```{tag}\n{}\n```", self.replacement);
            }
            Style::Plain | Style::Pretty => {
                let no_color = no_color || matches!(style, Style::Plain);
                print!("{}", if no_color { "" } else { "\x1b[1m" });
                self.kind.render(style, no_color, depth + 1);
                println!(
                    "{} at {}",
                    if no_color { "" } else { "\x1b[0m" },
                    color("\x1b[36m", &format!("{path}:{loc}"), no_color),
                );
                println!("{}", color("\x1b[31m", &self.original, no_color));
                println!("{}", color("\x1b[32m", &self.replacement, no_color));
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

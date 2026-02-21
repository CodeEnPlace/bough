use pollard_core::{Hash, MutationKind};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Clone, clap::ValueEnum)]
pub enum Style {
    Plain,
    Pretty,
    Json,
}

pub trait RenderOutput {
    fn render(&self, style: &Style);
}

#[derive(Serialize)]
pub struct MutationRecord {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
    pub kind: MutationKind,
    pub start_line: usize,
    pub start_char: usize,
    pub end_line: usize,
    pub end_char: usize,
    pub original: String,
    pub replacement: String,
}

impl RenderOutput for MutationRecord {
    fn render(&self, style: &Style) {
        match style {
            Style::Json => {
                println!("{}", serde_json::to_string(self).expect("failed to serialize"));
            }
            Style::Plain => {
                println!(
                    "{:?} {}:{}-{}:{} {} -> {}",
                    self.kind,
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                    self.original,
                    self.replacement,
                );
            }
            Style::Pretty => {
                println!(
                    "\x1b[1m{:?}\x1b[0m \x1b[36m{}:{}-{}:{}\x1b[0m \x1b[31m{}\x1b[0m -> \x1b[32m{}\x1b[0m",
                    self.kind,
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                    self.original,
                    self.replacement,
                );
            }
        }
    }
}

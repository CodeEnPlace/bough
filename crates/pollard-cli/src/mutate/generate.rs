use crate::io::{Action, Render, Report, color, hashed_path};
use crate::steps::expand_glob;
use pollard_core::config::LanguageId;
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::{
    Hash, Language, MutationKind, SourceFile, find_mutation_points,
    generate_mutation_substitutions,
};
use serde::Serialize;
use std::path::PathBuf;

fn generate_for_language<L: Language>(file: &SourceFile) -> Vec<GenerateReport>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    let mut records = Vec::new();
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(point) {
            records.push(GenerateReport {
                source_path: file.path().to_owned(),
                source_hash: file.hash().clone(),
                mutated_hash: mutated.hash().clone(),
                kind: point.kind.into(),
                start_line: point.span.start.line,
                start_char: point.span.start.char,
                end_line: point.span.end.line,
                end_char: point.span.end.char,
                original: file.content()[point.span.start.byte..point.span.end.byte].to_string(),
                replacement,
            });
        }
    }
    records
}

pub fn run(language: &LanguageId, pattern: &str) -> (Vec<Action>, Vec<GenerateReport>) {
    let paths = expand_glob(pattern);
    let mut records = Vec::new();
    for path in &paths {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_records = match language {
            LanguageId::Javascript => generate_for_language::<JavaScript>(&file),
            LanguageId::Typescript => generate_for_language::<TypeScript>(&file),
        };
        records.append(&mut file_records);
    }
    (vec![], records)
}

#[derive(Serialize)]
pub struct GenerateReport {
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

impl Render for GenerateReport {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        format!(
            "{} {} {} {}\n{}\n{}\n\n",
            color("\x1b[33m", &self.mutated_hash.to_string()),
            self.source_path.display(),
            color("\x1b[1m", &format!("{:?}", self.kind)),
            color(
                "\x1b[36m",
                &format!(
                    "{}:{}-{}:{}",
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                ),
            ),
            color("\x1b[31m", &self.original),
            color("\x1b[32m", &self.replacement),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

impl Report for GenerateReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.directories.report.join("mutate").join("generate")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "mutations")
    }

}

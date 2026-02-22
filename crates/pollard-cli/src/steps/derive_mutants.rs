use crate::io::{Action, Render, Report, Style, hashed_path};
use pollard_session::Session;
use crate::steps::{color, content_id};
use pollard_core::config::LanguageId;
use pollard_core::languages::javascript::JavaScript;
use pollard_core::languages::typescript::TypeScript;
use pollard_core::plan::{Plan, PlanEntry};
use pollard_core::{
    Language, MutationKind, SourceFile, find_mutation_points, generate_mutation_substitutions,
};
use serde::Serialize;
use std::path::PathBuf;

fn plan_entries_for_language<L: Language>(file: &SourceFile) -> Vec<PlanEntry>
where
    L::Kind: Copy + Into<MutationKind>,
{
    let points = find_mutation_points::<L>(file);
    let mut entries = Vec::new();
    for point in &points {
        for (replacement, mutated) in generate_mutation_substitutions::<L>(point) {
            entries.push(PlanEntry {
                source_path: file.path().to_owned(),
                source_hash: file.hash().clone(),
                mutated_hash: mutated.hash().clone(),
                kind: point.kind.into(),
                start_line: point.span.start.line,
                start_char: point.span.start.char,
                end_line: point.span.end.line,
                end_char: point.span.end.char,
                start_byte: point.span.start.byte,
                end_byte: point.span.end.byte,
                original: file.content()[point.span.start.byte..point.span.end.byte].to_string(),
                replacement,
            });
        }
    }
    entries
}

fn generate_plan(language: &LanguageId, files: &[PathBuf]) -> Plan {
    let mut entries = Vec::new();
    for path in files {
        let file = SourceFile::read(path).expect("failed to read input file");
        let mut file_entries = match language {
            LanguageId::Javascript => plan_entries_for_language::<JavaScript>(&file),
            LanguageId::Typescript => plan_entries_for_language::<TypeScript>(&file),
        };
        entries.append(&mut file_entries);
    }
    Plan { entries }
}

pub fn run(session: &Session, files: &[PathBuf]) -> (Vec<Action>, DeriveMutantsReport) {
    let plan = generate_plan(&session.language, files);
    let content = serde_json::to_string_pretty(&plan).expect("failed to serialize plan");
    let plan_path = session
        .working_dir
        .join(format!("{}.mutants.plan.json", content_id(&content)));

    let actions = vec![Action::WriteFile {
        path: plan_path.clone(),
        content,
    }];

    let report = DeriveMutantsReport {
        path: plan_path,
        count: plan.entries.len(),
    };

    (actions, report)
}

#[derive(Serialize)]
pub struct DeriveMutantsReport {
    pub path: PathBuf,
    pub count: usize,
}

impl Render for DeriveMutantsReport {
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
                    "Derived {} mutations, will write to {}",
                    color("\x1b[33m", &self.count.to_string(), no_color),
                    color("\x1b[36m", &self.path.display().to_string(), no_color),
                );
            }
            Style::Plain | Style::Markdown => {
                println!(
                    "Derived {} mutations, will write to {}",
                    self.count,
                    self.path.display()
                );
            }
        }
    }
}

impl Report for DeriveMutantsReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("step").join("derive-mutants")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "plan")
    }

}

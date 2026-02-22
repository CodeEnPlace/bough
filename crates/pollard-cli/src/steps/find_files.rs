use crate::io::{Action, Report, Style, hashed_path};
use crate::session::Session;
use crate::steps::expand_glob;
use serde::Serialize;
use std::path::PathBuf;

pub fn run(session: &Session) -> (Vec<Action>, FindFilesReport) {
    let paths = expand_glob(&session.files);

    let report = FindFilesReport {
        pattern: session.files.clone(),
        files: paths,
    };

    (vec![], report)
}

#[derive(Serialize)]
pub struct FindFilesReport {
    pub pattern: String,
    pub files: Vec<PathBuf>,
}

impl Report for FindFilesReport {
    fn get_dir(&self, session: &crate::session::Session) -> PathBuf {
        session.report_dir.join("step").join("find-files")
    }

    fn make_path(&self, session: &crate::session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "files")
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
                    "Found {} files matching {}",
                    crate::steps::color("\x1b[33m", &self.files.len().to_string(), no_color),
                    crate::steps::color("\x1b[36m", &self.pattern, no_color),
                );
                for f in &self.files {
                    println!("  {}", f.display());
                }
            }
            Style::Plain | Style::Markdown => {
                println!("Found {} files matching {}", self.files.len(), self.pattern);
                for f in &self.files {
                    println!("  {}", f.display());
                }
            }
        }
    }
}

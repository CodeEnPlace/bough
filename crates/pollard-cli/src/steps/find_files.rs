use crate::io::{Action, Render, Report, Style, hashed_path};
use pollard_session::Session;
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

impl Render for FindFilesReport {
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
                    "{} files match {}",
                    crate::io::color("\x1b[33m", &self.files.len().to_string(), no_color),
                    crate::io::color("\x1b[36m", &self.pattern, no_color),
                );
                for f in &self.files {
                    println!("  {}", f.display());
                }
            }
        }
    }
}

impl Report for FindFilesReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.report_dir.join("step").join("find-files")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "files")
    }

}

use crate::io::{Action, Render, Report, color, hashed_path};
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
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        let mut out = format!(
            "{} files match {}\n",
            color("\x1b[33m", &self.files.len().to_string()),
            color("\x1b[36m", &self.pattern),
        );
        for f in &self.files {
            out.push_str(&format!("  {}\n", f.display()));
        }
        out
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

impl Report for FindFilesReport {
    fn get_dir(&self, session: &pollard_session::Session) -> PathBuf {
        session.directories.report.join("step").join("find-files")
    }

    fn make_path(&self, session: &pollard_session::Session) -> PathBuf {
        let content = serde_json::to_string(self).expect("failed to serialize");
        hashed_path(&self.get_dir(session), &content, "files")
    }

}

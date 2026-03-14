use std::path::PathBuf;

use bough_core::{File, Session};
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, RESET, Render};

#[derive(Facet)]
pub struct ShowAllFiles(pub Vec<PathBuf>);

impl ShowAllFiles {
    pub fn run(config: Config) -> Box<Self> {
        let session = Session::new(config).expect("session creation");
        let base = session.base();
        let twigs = base.twigs().collect::<Vec<_>>();
        let files = twigs
            .iter()
            .map(|twig| File::new(base, &twig))
            .collect::<Vec<_>>();
        let paths = files.iter().map(|file| file.resolve()).collect();
        Box::new(Self(paths))
    }
}

impl Render for ShowAllFiles {
    fn markdown(&self) -> String {
        format!(
            "# Files in Base Directory\n\n\tThese files will be coppied into Workspace directories\n\n{}",
            self.0
                .iter()
                .map(|pb| format!("- {}", pb.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

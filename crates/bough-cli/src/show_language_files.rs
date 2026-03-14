use std::path::PathBuf;

use bough_core::{File, LanguageId, Session};
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, RESET, Render};

#[derive(Facet)]
pub struct ShowLanguageFiles(pub LanguageId, pub Vec<PathBuf>);

impl ShowLanguageFiles {
    pub fn run(config: Config, lang: LanguageId) -> Box<Self> {
        let session = Session::new(config).expect("session creation");
        let base = session.base();
        let twigs = base.mutant_twigs().collect::<Vec<_>>();
        let files = twigs
            .iter()
            .filter(|(l, _)| *l == lang)
            .map(|(_, twig)| File::new(base, &twig))
            .collect::<Vec<_>>();
        let paths = files.iter().map(|file| file.resolve()).collect();
        Box::new(Self(lang, paths))
    }
}

impl Render for ShowLanguageFiles {
    fn markdown(&self) -> String {
        format!(
            "# {:?} Files that will be Mutated\n\n{}",
            self.0,
            self.1
                .iter()
                .map(|pb| format!("- {}", pb.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
    fn terse(&self) -> String {
        self.1
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.1
            .iter()
            .map(|pb| format!("{PATH}{}{RESET}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

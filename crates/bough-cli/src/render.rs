use std::path::PathBuf;

use bough_core::LanguageId;

use crate::config::Format;

pub trait Render {
    fn markdown(&self) -> String;
    fn terse(&self) -> String;
    fn verbose(&self) -> String;
    fn render(&self, format: Format) -> String {
        match format {
            Format::Terse => self.terse(),
            Format::Verbose => self.verbose(),
            Format::Markdown => self.markdown(),
            Format::Json => todo!(),
        }
    }
}

pub struct Noop;
impl Render for Noop {
    fn markdown(&self) -> String {
        String::new()
    }

    fn terse(&self) -> String {
        todo!()
    }

    fn verbose(&self) -> String {
        todo!()
    }
}

pub struct BaseFiles(pub Vec<PathBuf>);
impl Render for BaseFiles {
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
            .map(|pb| format!("{}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(|pb| format!("{}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub struct MutantFiles(pub LanguageId, pub Vec<PathBuf>);
impl Render for MutantFiles {
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
            .map(|pb| format!("{}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join(" ")
    }
    fn verbose(&self) -> String {
        self.1
            .iter()
            .map(|pb| format!("{}", pb.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

use std::path::PathBuf;

pub trait Render {
    fn to_markdown(&self) -> String;
}

pub struct Noop;
impl Render for Noop {
    fn to_markdown(&self) -> String {
        String::new()
    }
}

pub struct BaseFiles(pub Vec<PathBuf>);
impl Render for BaseFiles {
    fn to_markdown(&self) -> String {
        format!(
            "# Files in Base Directory\n{}",
            self.0
                .iter()
                .map(|pb| format!("- {}", pb.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

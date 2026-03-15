use std::ops::Deref;
use std::path::PathBuf;

use bough_core::{File, Session};
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, RESET, TITLE, Render};

#[derive(Facet)]
pub struct ShowAllFiles(pub Vec<PathBuf>);

impl ShowAllFiles {
    pub fn run(session: impl Deref<Target = Session<Config>>) -> Box<Self> {
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
        let list = self
            .0
            .iter()
            .map(|p| format!("- {PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{TITLE}# All Files{RESET}\n\n{list}")
    }

    fn terse(&self) -> String {
        self.0
            .iter()
            .map(|p| format!("{PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn verbose(&self) -> String {
        self.0
            .iter()
            .map(|p| format!("{PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn json(&self) -> String {
        let items: Vec<String> = self.0.iter().map(|p| format!("\"{}\"", p.display())).collect();
        format!("[{}]", items.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ShowAllFiles {
        ShowAllFiles(vec![
            PathBuf::from("src/main.ts"),
            PathBuf::from("src/lib.ts"),
        ])
    }

    #[test]
    fn markdown() {
        let plain = fixture()
            .markdown()
            .replace(TITLE, "")
            .replace(PATH, "")
            .replace(RESET, "");
        assert_eq!(
            plain,
            "\
# All Files

- src/main.ts
- src/lib.ts"
        );
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(!out.contains('\n'));
        let plain = out.replace(PATH, "").replace(RESET, "");
        assert_eq!(plain, "src/main.ts src/lib.ts");
    }

    #[test]
    fn verbose() {
        let plain = fixture()
            .verbose()
            .replace(PATH, "")
            .replace(RESET, "");
        assert_eq!(
            plain,
            "\
src/main.ts
src/lib.ts"
        );
    }

    #[test]
    fn json() {
        assert_eq!(
            fixture().json(),
            r#"["src/main.ts","src/lib.ts"]"#
        );
    }
}



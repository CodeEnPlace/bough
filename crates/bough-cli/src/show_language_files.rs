use std::ops::Deref;
use std::path::PathBuf;

use bough_core::{File, LanguageId, Session};
use facet::Facet;

use crate::config::Config;
use crate::render::{PATH, RESET, TITLE, Render};

#[derive(Facet)]
pub struct ShowLanguageFiles(pub LanguageId, pub Vec<PathBuf>);

impl ShowLanguageFiles {
    pub fn run(session: impl Deref<Target = Session<Config>>, lang: LanguageId) -> Box<Self> {
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
        let list = self
            .1
            .iter()
            .map(|p| format!("- {PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{TITLE}# {} Files{RESET}\n\n{list}", self.0.markdown())
    }

    fn terse(&self) -> String {
        let paths = self
            .1
            .iter()
            .map(|p| format!("{PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join(" ");
        format!("{} {paths}", self.0.terse())
    }

    fn verbose(&self) -> String {
        let list = self
            .1
            .iter()
            .map(|p| format!("  {PATH}{}{RESET}", p.display()))
            .collect::<Vec<_>>()
            .join("\n");
        format!("{}\n{list}", self.0.verbose())
    }

    fn json(&self) -> String {
        facet_json::to_string(self).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> ShowLanguageFiles {
        ShowLanguageFiles(
            LanguageId::Typescript,
            vec![
                PathBuf::from("src/main.ts"),
                PathBuf::from("src/lib.ts"),
            ],
        )
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
# TypeScript Files

- src/main.ts
- src/lib.ts"
        );
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(!out.contains('\n'));
    }

    #[test]
    fn verbose() {
        let plain = fixture()
            .verbose()
            .replace(crate::render::LANG, "")
            .replace(PATH, "")
            .replace(RESET, "");
        assert_eq!(
            plain,
            "\
TypeScript
  src/main.ts
  src/lib.ts"
        );
    }

    #[test]
    fn json() {
        assert_eq!(
            fixture().json(),
            r#"["ts",["src/main.ts","src/lib.ts"]]"#
        );
    }
}



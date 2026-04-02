use std::ops::DerefMut;

use bough_lib::Session;

use crate::config::Config;
use crate::render::{MUTATION, RESET, Render, TITLE};

pub struct StepTendState {
    pub added: Vec<bough_core::MutationHash>,
    pub removed: Vec<bough_core::MutationHash>,
}

impl StepTendState {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>) -> Box<Self> {
        let added = session
            .tend_add_missing_states()
            .expect("tend add missing states");
        let removed = session
            .tend_remove_stale_states()
            .expect("tend remove stale states");
        Box::new(Self { added, removed })
    }
}

impl Render for StepTendState {
    fn markdown(&self) -> String {
        format!(
            "{TITLE}# Tend State{RESET}\n\n- Added: {}\n- Removed: {}",
            self.added.len(),
            self.removed.len(),
        )
    }

    fn terse(&self) -> String {
        format!("+{} -{}", self.added.len(), self.removed.len())
    }

    fn verbose(&self) -> String {
        let mut out = format!(
            "{TITLE}Tend State{RESET} +{} -{}",
            self.added.len(),
            self.removed.len(),
        );
        for h in &self.added {
            out.push_str(&format!("\n  {MUTATION}+{h}{RESET}"));
        }
        for h in &self.removed {
            out.push_str(&format!("\n  {MUTATION}-{h}{RESET}"));
        }
        out
    }

    fn json(&self) -> String {
        let added: Vec<String> = self.added.iter().map(|h| format!("\"{h}\"")).collect();
        let removed: Vec<String> = self.removed.iter().map(|h| format!("\"{h}\"")).collect();
        format!(
            r#"{{"added":[{}],"removed":[{}]}}"#,
            added.join(","),
            removed.join(","),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_core::*;
    use bough_typed_hash::TypedHashable;

    fn fixture() -> StepTendState {
        let mutant = Mutant::new(
            LanguageId::Typescript,
            Twig::new("src/main.ts".into()).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
            Span::new(Point::new(0, 0, 0), Point::new(2, 3, 20)),
        );
        let hash = MutationIter::new(&mutant, &bough_lang_javascript::JavascriptDriver)
            .next()
            .unwrap()
            .hash()
            .expect("hash");
        StepTendState {
            added: vec![hash],
            removed: vec![],
        }
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert_eq!(plain, "# Tend State\n\n- Added: 1\n- Removed: 0");
    }

    #[test]
    fn terse() {
        assert_eq!(fixture().terse(), "+1 -0");
    }

    #[test]
    fn verbose() {
        let out = fixture().verbose();
        let plain = out
            .replace(TITLE, "")
            .replace(MUTATION, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Tend State +1 -0\n"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.contains(r#""added":["#));
        assert!(out.contains(r#""removed":[]"#));
    }
}

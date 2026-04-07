use std::ops::DerefMut;

use bough_lib::Session;

use crate::config::Config;
use crate::render::{RESET, Render, TITLE, WORKSPACE};

pub struct StepTendWorkspaces {
    pub workspace_ids: Vec<bough_dirs::WorkId>,
}

impl StepTendWorkspaces {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, config: &Config) -> Box<Self> {
        let workers = config.workers as usize;
        let workspace_ids = session.tend_workspaces(workers).expect("tend workspaces");
        Box::new(Self { workspace_ids })
    }
}

impl Render for StepTendWorkspaces {
    fn markdown(&self) -> String {
        let list = self
            .workspace_ids
            .iter()
            .map(|id| format!("- `{id}`"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}# Tend Workspaces{RESET}\n\n{} total\n\n{list}",
            self.workspace_ids.len(),
        )
    }

    fn terse(&self) -> String {
        self.workspace_ids
            .iter()
            .map(|id| format!("{WORKSPACE}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn verbose(&self) -> String {
        let list = self
            .workspace_ids
            .iter()
            .map(|id| format!("  {WORKSPACE}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}Tend Workspaces{RESET} ({} total)\n{list}",
            self.workspace_ids.len(),
        )
    }

    fn json(&self) -> String {
        let items: Vec<String> = self
            .workspace_ids
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect();
        format!("[{}]", items.join(","))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_dirs::WorkId;

    fn fixture() -> StepTendWorkspaces {
        StepTendWorkspaces {
            workspace_ids: vec![
                WorkId::parse("aaaa1111").unwrap(),
                WorkId::parse("bbbb2222").unwrap(),
            ],
        }
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# Tend Workspaces\n\n2 total"));
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
            .replace(TITLE, "")
            .replace(WORKSPACE, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Tend Workspaces (2 total)"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.starts_with('['));
        assert!(out.contains("aaaa1111"));
    }
}

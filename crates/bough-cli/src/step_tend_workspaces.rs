use bough_core::Session;

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepTendWorkspaces {
    pub workspace_ids: Vec<bough_core::WorkspaceId>,
}

impl StepTendWorkspaces {
    pub fn run(config: Config) -> Box<Self> {
        let mut session = Session::new(config.clone()).expect("session creation");
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
            "# Tend Workspaces\n\n{} total\n\n{list}",
            self.workspace_ids.len()
        )
    }

    fn terse(&self) -> String {
        self.workspace_ids
            .iter()
            .map(|id| format!("{HASH}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn verbose(&self) -> String {
        let list = self
            .workspace_ids
            .iter()
            .map(|id| format!("  {HASH}{id}{RESET}"))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "{TITLE}Tend Workspaces{RESET}\n\n{} total\n\n{list}",
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

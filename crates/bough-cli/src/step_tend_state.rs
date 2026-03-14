use bough_core::Session;

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render, hash_list_json};

pub struct StepTendState {
    pub added: Vec<bough_core::MutationHash>,
    pub removed: Vec<bough_core::MutationHash>,
}

impl StepTendState {
    pub fn run(config: Config) -> Box<Self> {
        let mut session = Session::new(config).expect("session creation");
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
            "# Tend State\n\n- Added: {}\n- Removed: {}",
            self.added.len(),
            self.removed.len(),
        )
    }

    fn terse(&self) -> String {
        format!("+{} -{}", self.added.len(), self.removed.len())
    }

    fn verbose(&self) -> String {
        let mut out = format!(
            "{TITLE}Tend State{RESET}\n\nAdded: {}, Removed: {}",
            self.added.len(),
            self.removed.len(),
        );
        for h in &self.added {
            out.push_str(&format!("\n  {HASH}+{h}{RESET}"));
        }
        for h in &self.removed {
            out.push_str(&format!("\n  {HASH}-{h}{RESET}"));
        }
        out
    }

    fn json(&self) -> String {
        format!(
            r#"{{"added":{},"removed":{}}}"#,
            hash_list_json(&self.added),
            hash_list_json(&self.removed),
        )
    }
}

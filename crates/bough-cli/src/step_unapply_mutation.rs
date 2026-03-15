use std::ops::Deref;

use bough_core::Session;

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepUnapplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl StepUnapplyMutation {
    pub fn run(session: impl Deref<Target = Session<Config>>, workspace_id: &str, mutation_hash: &str) -> Box<Self> {
        let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
        let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
        workspace.revert_mutant().expect("unapply mutation");
        Box::new(Self {
            workspace_id: wid,
            mutation_hash: mutation_hash.to_string(),
        })
    }
}

impl Render for StepUnapplyMutation {
    fn markdown(&self) -> String {
        format!(
            "# Unapply Mutation\n\n- Workspace: `{}`\n- Mutation: `{}`",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {HASH}{}{RESET}",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Unapply Mutation{RESET} {HASH}{}{RESET} from {HASH}{}{RESET}",
            self.mutation_hash, self.workspace_id,
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","mutation_hash":"{}"}}"#,
            self.workspace_id, self.mutation_hash,
        )
    }
}

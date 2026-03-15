use bough_typed_hash::TypedHashable;

use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepUnapplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl StepUnapplyMutation {
    pub fn run(workspace: &mut bough_core::Workspace) -> Result<Box<Self>, bough_core::WorkspaceError> {
        let mutation_hash = workspace.active()
            .map(|a| format!("{}", a.mutation().hash().expect("hash")))
            .unwrap_or_default();
        workspace.revert_mutant()?;
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            mutation_hash,
        }))
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

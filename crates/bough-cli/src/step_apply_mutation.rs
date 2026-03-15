use bough_typed_hash::TypedHashable;

use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepApplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl StepApplyMutation {
    pub fn run(workspace: &mut bough_core::Workspace, mutation: &bough_core::Mutation) -> Result<Box<Self>, bough_core::WorkspaceError> {
        workspace.write_mutant(mutation)?;
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            mutation_hash: mutation.hash().expect("hash").to_string(),
        }))
    }
}

impl Render for StepApplyMutation {
    fn markdown(&self) -> String {
        format!(
            "# Apply Mutation\n\n- Workspace: `{}`\n- Mutation: `{}`",
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
            "{TITLE}Apply Mutation{RESET} {HASH}{}{RESET} to {HASH}{}{RESET}",
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

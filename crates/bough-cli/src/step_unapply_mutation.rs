use bough_typed_hash::TypedHashable;

use crate::render::{MUTATION, RESET, Render, TITLE, WORKSPACE};

pub struct StepUnapplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl StepUnapplyMutation {
    pub fn run(
        workspace: &mut bough_core::Workspace,
    ) -> Result<Box<Self>, bough_core::WorkspaceError> {
        let mutation_hash = workspace
            .active()
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
            "{TITLE}# Unapply Mutation{RESET}\n\n- Workspace: `{}`\n- Mutation: `{}`",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn terse(&self) -> String {
        format!(
            "{WORKSPACE}{}{RESET} {MUTATION}{}{RESET}",
            self.workspace_id, self.mutation_hash,
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Unapply Mutation{RESET} {MUTATION}{}{RESET} from {WORKSPACE}{}{RESET}",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> StepUnapplyMutation {
        StepUnapplyMutation {
            workspace_id: bough_core::WorkspaceId::parse("aaaa1111").unwrap(),
            mutation_hash: "abcdef12".to_string(),
        }
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.contains("Workspace: `aaaa1111`"));
        assert!(plain.contains("Mutation: `abcdef12`"));
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
            .replace(MUTATION, "")
            .replace(WORKSPACE, "")
            .replace(RESET, "");
        assert_eq!(plain, "Unapply Mutation abcdef12 from aaaa1111");
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.contains(r#""workspace_id":"aaaa1111""#));
        assert!(out.contains(r#""mutation_hash":"abcdef12""#));
    }
}

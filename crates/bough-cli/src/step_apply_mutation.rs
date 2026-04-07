use bough_typed_hash::TypedHashable;

use crate::render::{MUTATION, RESET, Render, TITLE, WORKSPACE};

pub struct StepApplyMutation {
    pub workspace_id: bough_dirs::WorkId,
    pub mutation_hash: String,
}

impl StepApplyMutation {
    pub fn run(
        workspace: &mut bough_dirs::Work,
        mutation: &bough_core::Mutation,
    ) -> Result<Box<Self>, bough_dirs::Error> {
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
            "{TITLE}# Apply Mutation{RESET}\n\n- Workspace: `{}`\n- Mutation: `{}`",
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
            "{TITLE}Apply Mutation{RESET} {MUTATION}{}{RESET} to {WORKSPACE}{}{RESET}",
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

    fn fixture() -> StepApplyMutation {
        StepApplyMutation {
            workspace_id: bough_dirs::WorkId::parse("aaaa1111").unwrap(),
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
        assert_eq!(plain, "Apply Mutation abcdef12 to aaaa1111");
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.contains(r#""workspace_id":"aaaa1111""#));
        assert!(out.contains(r#""mutation_hash":"abcdef12""#));
    }
}

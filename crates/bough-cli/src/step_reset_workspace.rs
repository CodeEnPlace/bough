use crate::config::Config;
use crate::render::{RESET, Render, TITLE, WORKSPACE};

pub struct StepResetWorkspace {
    pub workspace_id: bough_lib::WorkspaceId,
    pub outcome: bough_lib::PhaseOutcome,
}

impl StepResetWorkspace {
    pub fn run(
        workspace: &bough_lib::Workspace,
        config: &Config,
        timeout: Option<chrono::Duration>,
    ) -> Result<Box<Self>, bough_lib::PhaseError> {
        let outcome = workspace.run_reset(config, timeout)?;
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            outcome,
        }))
    }
}

impl Render for StepResetWorkspace {
    fn markdown(&self) -> String {
        format!(
            "{TITLE}# Reset Workspace{RESET} `{}`\n\n{}",
            self.workspace_id,
            self.outcome.markdown(),
        )
    }

    fn terse(&self) -> String {
        format!(
            "{WORKSPACE}{}{RESET} {}",
            self.workspace_id,
            self.outcome.terse(),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Reset Workspace{RESET} {WORKSPACE}{}{RESET}\n\n{}",
            self.workspace_id,
            self.outcome.verbose(),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","outcome":{}}}"#,
            self.workspace_id,
            self.outcome.json(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_lib::{PhaseOutcome, WorkspaceId};

    fn fixture() -> StepResetWorkspace {
        StepResetWorkspace {
            workspace_id: WorkspaceId::parse("aaaa1111").unwrap(),
            outcome: PhaseOutcome::Completed {
                exit_code: 0,
                duration: std::time::Duration::from_secs(1),
                stdout: vec![],
                stderr: vec![],
            },
        }
    }

    #[test]
    fn markdown() {
        let plain = fixture().markdown().replace(TITLE, "").replace(RESET, "");
        assert!(plain.starts_with("# Reset Workspace `aaaa1111`"));
    }

    #[test]
    fn terse() {
        let out = fixture().terse();
        assert!(!out.contains('\n'));
        assert!(out.contains("aaaa1111"));
    }

    #[test]
    fn verbose() {
        let plain = fixture()
            .verbose()
            .replace(TITLE, "")
            .replace(WORKSPACE, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Reset Workspace aaaa1111"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.contains(r#""workspace_id":"aaaa1111""#));
    }
}

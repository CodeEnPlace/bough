use bough_typed_hash::TypedHashable;

use crate::config::Config;
use crate::render::{MUTATION, RESET, Render, TITLE, WORKSPACE};

pub struct StepTestMutation {
    pub workspace_id: bough_lib::WorkspaceId,
    pub mutation_hash: String,
    pub status: &'static str,
    pub status_value: bough_lib::Status,
    pub duration: std::time::Duration,
}

impl StepTestMutation {
    pub fn run(
        workspace: &bough_lib::Workspace,
        config: &Config,
        mutation: &bough_core::Mutation,
        timeout: Option<chrono::Duration>,
    ) -> Result<Box<Self>, bough_lib::PhaseError> {
        let outcome = workspace.run_test(config, timeout)?;
        let duration = outcome.duration();
        let (status_value, status_str) = match outcome {
            bough_lib::PhaseOutcome::TimedOut { .. } => (bough_lib::Status::Timeout, "timeout"),
            bough_lib::PhaseOutcome::Completed { exit_code, .. } if exit_code != 0 => {
                (bough_lib::Status::Caught, "caught")
            }
            bough_lib::PhaseOutcome::Completed { .. } => (bough_lib::Status::Missed, "missed"),
        };
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            mutation_hash: mutation.hash().expect("hash").to_string(),
            status: status_str,
            status_value,
            duration,
        }))
    }
}

impl Render for StepTestMutation {
    fn markdown(&self) -> String {
        format!(
            "{TITLE}# Test Mutation{RESET} `{}` in `{}`\n\n- Status: {}\n- Duration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status_value.markdown(),
            self.duration.as_secs_f64(),
        )
    }

    fn terse(&self) -> String {
        format!(
            "{WORKSPACE}{}{RESET} {MUTATION}{}{RESET} {} {:.2}s",
            self.workspace_id,
            self.mutation_hash,
            self.status_value.terse(),
            self.duration.as_secs_f64(),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Test Mutation{RESET} {MUTATION}{}{RESET} in {WORKSPACE}{}{RESET}\n\nStatus: {}\nDuration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status_value.verbose(),
            self.duration.as_secs_f64(),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","mutation_hash":"{}","status":"{}","duration_secs":{:.3}}}"#,
            self.workspace_id,
            self.mutation_hash,
            self.status,
            self.duration.as_secs_f64(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> StepTestMutation {
        StepTestMutation {
            workspace_id: bough_lib::WorkspaceId::parse("aaaa1111").unwrap(),
            mutation_hash: "abcdef12".to_string(),
            status: "caught",
            status_value: bough_lib::Status::Caught,
            duration: std::time::Duration::from_millis(500),
        }
    }

    #[test]
    fn markdown() {
        let out = fixture().markdown();
        assert!(out.contains("abcdef12"));
        assert!(out.contains("aaaa1111"));
        assert!(out.contains("0.50s"));
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
            .replace(MUTATION, "")
            .replace(WORKSPACE, "")
            .replace(RESET, "");
        assert!(plain.starts_with("Test Mutation abcdef12 in aaaa1111"));
    }

    #[test]
    fn json() {
        let out = fixture().json();
        assert!(out.contains(r#""status":"caught""#));
        assert!(out.contains(r#""duration_secs":0.500"#));
    }
}

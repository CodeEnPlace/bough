use bough_typed_hash::TypedHashable;

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepTestMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
    pub status: &'static str,
    pub status_value: bough_core::Status,
    pub duration: std::time::Duration,
}

impl StepTestMutation {
    pub fn run(workspace: &bough_core::Workspace, config: &Config, mutation: &bough_core::Mutation, timeout: Option<std::time::Duration>) -> Result<Box<Self>, bough_core::PhaseError> {
        let outcome = workspace.run_test(config, timeout)?;
        let (status_value, status_str) = if outcome.exit_code() != 0 {
            (bough_core::Status::Caught, "caught")
        } else {
            (bough_core::Status::Missed, "missed")
        };
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            mutation_hash: mutation.hash().expect("hash").to_string(),
            status: status_str,
            status_value,
            duration: outcome.duration(),
        }))
    }
}

impl Render for StepTestMutation {
    fn markdown(&self) -> String {
        format!(
            "# Test Mutation `{}` in `{}`\n\n- Status: {}\n- Duration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status,
            self.duration.as_secs_f64(),
        )
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {HASH}{}{RESET} {} {:.2}s",
            self.workspace_id,
            self.mutation_hash,
            self.status,
            self.duration.as_secs_f64(),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Test Mutation{RESET} {HASH}{}{RESET} in {HASH}{}{RESET}\n\nStatus: {}\nDuration: {:.2}s",
            self.mutation_hash,
            self.workspace_id,
            self.status,
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

use crate::config::Config;
use crate::render::{
    HASH, RESET, TITLE, Render, fmt_phase_outcome_json, fmt_phase_outcome_terse,
    fmt_phase_outcome_verbose,
};

pub struct StepInitWorkspace {
    pub workspace_id: bough_core::WorkspaceId,
    pub outcome: bough_core::PhaseOutcome,
}

impl StepInitWorkspace {
    pub fn run(workspace: &bough_core::Workspace, config: &Config, timeout: Option<std::time::Duration>) -> Result<Box<Self>, bough_core::PhaseError> {
        let outcome = workspace.run_init(config, timeout)?;
        Ok(Box::new(Self {
            workspace_id: workspace.id().clone(),
            outcome,
        }))
    }
}

impl Render for StepInitWorkspace {
    fn markdown(&self) -> String {
        let stdout = String::from_utf8_lossy(self.outcome.stdout());
        let stderr = String::from_utf8_lossy(self.outcome.stderr());
        let mut out = format!(
            "# Init Workspace `{}`\n\n- Exit: {}\n- Duration: {:.2}s\n- Timed out: {}",
            self.workspace_id,
            self.outcome.exit_code(),
            self.outcome.duration().as_secs_f64(),
            self.outcome.timed_out(),
        );
        if !stdout.is_empty() {
            out.push_str(&format!("\n\n## stdout\n\n```\n{stdout}\n```"));
        }
        if !stderr.is_empty() {
            out.push_str(&format!("\n\n## stderr\n\n```\n{stderr}\n```"));
        }
        out
    }

    fn terse(&self) -> String {
        format!(
            "{HASH}{}{RESET} {}",
            self.workspace_id,
            fmt_phase_outcome_terse(&self.outcome),
        )
    }

    fn verbose(&self) -> String {
        format!(
            "{TITLE}Init Workspace{RESET} {HASH}{}{RESET}\n\n{}",
            self.workspace_id,
            fmt_phase_outcome_verbose(&self.outcome),
        )
    }

    fn json(&self) -> String {
        format!(
            r#"{{"workspace_id":"{}","outcome":{}}}"#,
            self.workspace_id,
            fmt_phase_outcome_json(&self.outcome),
        )
    }
}

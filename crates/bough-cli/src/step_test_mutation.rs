use std::ops::DerefMut;

use bough_core::Session;
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepTestMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
    pub status: &'static str,
    pub duration: std::time::Duration,
}

impl StepTestMutation {
    pub fn run(mut session: impl DerefMut<Target = Session<Config>>, config: &Config, workspace_id: &str, mutation_hash: &str) -> Box<Self> {
        let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan");
        let mutation = find_mutation_by_hash(mutation_hash, mutations);
        let hash_str = mutation.hash().expect("hash").to_string();
        let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
        workspace.write_mutant(&mutation).expect("apply mutation");
        let outcome = workspace
            .run_test(config, None)
            .expect("test mutation");
        workspace.revert_mutant().expect("revert mutation");
        let status = if outcome.exit_code() != 0 {
            bough_core::Status::Caught
        } else {
            bough_core::Status::Missed
        };
        let status_str = if outcome.exit_code() != 0 {
            "caught"
        } else {
            "missed"
        };
        session.set_state(&mutation, status).expect("set state");
        Box::new(Self {
            workspace_id: wid,
            mutation_hash: hash_str,
            status: status_str,
            duration: outcome.duration(),
        })
    }
}

fn find_mutation_by_hash(hash: &str, mutations: Vec<bough_core::Mutation>) -> bough_core::Mutation {
    let unvalidated = UnvalidatedHash::new(hash.to_string());
    let hashes: Vec<_> = mutations
        .iter()
        .map(|m| m.hash().expect("hashing should not fail"))
        .collect();
    let matched = unvalidated
        .validate(&hashes)
        .expect("hash resolution failed");
    let matched_bytes = matched.as_bytes();
    mutations
        .into_iter()
        .find(|m| m.hash().unwrap().as_bytes() == matched_bytes)
        .unwrap()
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

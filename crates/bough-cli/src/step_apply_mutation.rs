use std::ops::Deref;

use bough_core::Session;
use bough_typed_hash::{TypedHash, TypedHashable, UnvalidatedHash};

use crate::config::Config;
use crate::render::{HASH, RESET, TITLE, Render};

pub struct StepApplyMutation {
    pub workspace_id: bough_core::WorkspaceId,
    pub mutation_hash: String,
}

impl StepApplyMutation {
    pub fn run(session: impl Deref<Target = Session<Config>>, workspace_id: &str, mutation_hash: &str) -> Box<Self> {
        let wid = bough_core::WorkspaceId::parse(workspace_id).expect("invalid workspace id");
        let base = session.base();
        let mutations: Vec<_> = base
            .mutations()
            .collect::<Result<Vec<_>, _>>()
            .expect("mutation scan");
        let mutation = find_mutation_by_hash(mutation_hash, mutations);
        let mut workspace = session.bind_workspace(&wid).expect("bind workspace");
        workspace.write_mutant(&mutation).expect("apply mutation");
        let hash_str = mutation.hash().expect("hash").to_string();
        Box::new(Self {
            workspace_id: wid,
            mutation_hash: hash_str,
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

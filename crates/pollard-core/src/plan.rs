use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::{Hash, MutationKind};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
    pub kind: MutationKind,
    pub start_line: usize,
    pub start_char: usize,
    pub end_line: usize,
    pub end_char: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub original: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
}

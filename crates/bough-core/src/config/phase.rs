use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseMetaConfig {
    pub(crate) pwd: Option<PathBuf>,
    pub(crate) timeout: TimeoutConfig,
    pub(crate) env: HashMap<String, String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhaseConfig {
    #[serde(flatten)]
    pub(crate) meta: PhaseMetaConfig,
    pub(crate) commands: Vec<String>,
}

impl PhaseConfig {
    pub fn commands(&self) -> &[String] {
        &self.commands
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    pub(crate) absolute: Option<u64>,
    pub(crate) relative: Option<u64>,
}

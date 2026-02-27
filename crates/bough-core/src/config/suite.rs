use crate::languages::LanguageId;
use crate::phase::Phase;
use crate::Outcome;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use super::phase::PhaseConfig;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OrderingConfig {
    #[default]
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SuiteConfig {
    pub(crate) ordering: OrderingConfig,
    pub(crate) treat_timeouts_as: Outcome,
    pub(crate) test_ids: Option<TestIdsConfig>,
    #[serde(flatten)]
    pub(crate) meta_defaults: super::phase::PhaseMetaConfig,
    #[serde(flatten)]
    pub(crate) phases: HashMap<Phase, PhaseConfig>,
    #[serde(default)]
    pub(crate) mutate: HashMap<LanguageId, MutateLanguageConfig>,
}

impl SuiteConfig {
    pub fn get_phase_config(&self, phase: Phase) -> Option<&PhaseConfig> {
        self.phases.get(&phase)
    }
}

impl Default for SuiteConfig {
    fn default() -> Self {
        Self {
            ordering: OrderingConfig::default(),
            treat_timeouts_as: Outcome::Caught,
            test_ids: None,
            meta_defaults: Default::default(),
            phases: HashMap::new(),
            mutate: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestIdsConfig {
    pub(crate) get_all: String,
    pub(crate) get_failed: String,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateLanguageConfig {
    pub(crate) files: FileSourceConfig,
    pub(crate) mutants: MutantFilterConfig,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileSourceConfig {
    pub(crate) include: Vec<String>,
    pub(crate) exclude: Vec<String>,
    pub(crate) ignore_files: Vec<PathBuf>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutantFilterConfig {
    pub(crate) skip: Vec<MutantSkipConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MutantSkipConfig {
    Query { query: String },
    Kind { kind: HashMap<String, String> },
}

use facet::Facet;

use crate::PhaseOverrides;

#[derive(Facet, Debug, Clone)]
pub struct TestPhaseConfig {
    pub cmd: String,

    #[facet(flatten, default)]
    pub overrides: PhaseOverrides,
}

use facet::Facet;

use crate::PhaseOverrides;

#[derive(Facet, Debug, Clone)]
pub struct PhaseConfig {
    #[facet(default)]
    pub cmd: Option<String>,

    #[facet(flatten, default)]
    pub overrides: PhaseOverrides,
}

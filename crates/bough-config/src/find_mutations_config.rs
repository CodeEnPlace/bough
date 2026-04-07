use facet::Facet;

use crate::Factor;

#[derive(Facet, Debug, Clone)]
pub struct FindMutationsConfig {
    #[facet(default = 1)]
    pub number: usize,

    #[facet(default = 1)]
    pub number_per_file: usize,

    #[facet(default = vec![Factor::EncompasingMissedMutationsCount, Factor::TSNodeDepth])]
    pub factors: Vec<Factor>,
}

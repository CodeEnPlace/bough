use facet::Facet;

#[derive(Facet, Debug, Clone)]
pub struct TimeoutConfig {
    #[facet(default)]
    pub absolute: Option<u64>,

    #[facet(default)]
    pub relative: Option<f64>,
}

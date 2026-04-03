use facet::Facet;

#[derive(Facet, Debug, Clone, Default)]
pub struct LanguageSkipConfig {
    #[facet(default)]
    pub query: Vec<String>,
}

use facet::Facet;

use crate::LanguageSkipConfig;

#[derive(Facet, Debug, Clone)]
pub struct LanguageConfig {
    pub include: Vec<String>,

    pub exclude: Vec<String>,

    #[facet(default)]
    pub skip: Option<LanguageSkipConfig>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum LanguageId {
    #[serde(alias = "js")]
    #[value(alias = "js")]
    Javascript,
    #[serde(alias = "ts")]
    #[value(alias = "ts")]
    Typescript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Vcs {
    Git,
    Jj,
    Mercurial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    Random,
    Alphabetical,
}

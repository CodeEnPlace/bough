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
pub enum VcsKind {
    None,
    Git,
    Jj,
    Mercurial,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Vcs {
    None,
    Git { commit: String },
    Jj { rev: String },
    Mercurial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

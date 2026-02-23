use crate::Outcome;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VcsConfig {
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub vcs: Option<VcsConfig>,
    pub parallelism: Option<u32>,
    pub ordering: Option<Ordering>,
    pub dirs: Option<Dirs>,
    #[serde(flatten)]
    pub runners: HashMap<String, Runner>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dirs {
    pub working: Option<String>,
    pub state: Option<String>,
    pub logs: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Runner {
    pub pwd: Option<String>,
    pub treat_timeouts_as: Option<Outcome>,
    pub init: Option<Phase>,
    pub reset: Option<Phase>,
    pub test: Option<Phase>,
    #[serde(default)]
    pub mutate: HashMap<String, MutateLanguage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Phase {
    pub pwd: Option<String>,
    pub timeout: Option<Timeout>,
    pub env: Option<HashMap<String, String>>,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Timeout {
    pub absolute: Option<u64>,
    pub relative: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutateLanguage {
    pub files: Option<FileFilter>,
    pub mutants: Option<MutantFilter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileFilter {
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutantFilter {
    #[serde(default)]
    pub skip: Vec<MutantSkip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MutantSkip {
    Lisp { lisp: String },
    Kind { kind: HashMap<String, String> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_ideal_config() {
        let toml_str = include_str!("ideal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse ideal config");

        assert_eq!(
            config.vcs,
            Some(VcsConfig::Jj {
                rev: "trunk()".into()
            })
        );
        assert_eq!(config.parallelism, Some(1));
        assert_eq!(config.ordering, Some(Ordering::Random));

        let dirs = config.dirs.as_ref().unwrap();
        assert_eq!(dirs.working.as_deref(), Some("/tmp/bough/work"));
        assert_eq!(dirs.state.as_deref(), Some("/tmp/bough/state"));
        assert_eq!(dirs.logs.as_deref(), Some("/tmp/bough/logs"));

        let vitest = &config.runners["vitest"];
        assert_eq!(vitest.treat_timeouts_as, Some(Outcome::Missed));

        let init = vitest.init.as_ref().unwrap();
        assert_eq!(init.pwd.as_deref(), Some("./examples/vitest"));
        assert_eq!(init.commands, vec!["npm install"]);

        let test = vitest.test.as_ref().unwrap();
        assert_eq!(test.timeout.as_ref().unwrap().absolute, Some(30));
        assert_eq!(test.timeout.as_ref().unwrap().relative, Some(3));
        assert_eq!(test.env.as_ref().unwrap()["NODE_ENV"], "production");
        assert_eq!(test.commands, vec!["npx run build", "npx run test"]);

        let js_mutate = &vitest.mutate["js"];
        let js_files = js_mutate.files.as_ref().unwrap();
        assert_eq!(js_files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_files.exclude, vec!["**/*__mocks__*"]);

        let js_mutants = js_mutate.mutants.as_ref().unwrap();
        assert_eq!(js_mutants.skip.len(), 2);

        let cargo = &config.runners["cargo"];
        assert_eq!(cargo.pwd.as_deref(), Some("./examples/cargo"));
        let cargo_test = cargo.test.as_ref().unwrap();
        assert_eq!(cargo_test.commands, vec!["cargo test"]);
    }
}

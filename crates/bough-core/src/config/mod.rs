use crate::Outcome;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum VcsConfig {
    #[default]
    None,
    Git {
        commit: String,
    },
    Jj {
        rev: String,
    },
    Mercurial,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Ordering {
    #[default]
    Random,
    Alphabetical,
    MissedFirst,
    CaughtFirst,
    NewestFirst,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub vcs: VcsConfig,
    pub parallelism: u32,
    pub ordering: Ordering,
    pub dirs: Dirs,
    #[serde(flatten)]
    pub runners: HashMap<String, Runner>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vcs: VcsConfig::default(),
            parallelism: 1,
            ordering: Ordering::default(),
            dirs: Dirs::default(),
            runners: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dirs {
    pub working: String,
    pub state: String,
    pub logs: String,
}

impl Default for Dirs {
    fn default() -> Self {
        Self {
            working: "/tmp/bough/work".into(),
            state: "./bough/state".into(),
            logs: "/tmp/bough/logs".into(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Runner {
    pub pwd: Option<String>,
    pub treat_timeouts_as: Outcome,
    pub init: Option<Phase>,
    pub reset: Option<Phase>,
    pub test: Option<Phase>,
    pub mutate: HashMap<String, MutateLanguage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Phase {
    pub pwd: Option<String>,
    pub timeout: Timeout,
    pub env: HashMap<String, String>,
    pub commands: Vec<String>,
}

impl Default for Phase {
    fn default() -> Self {
        Self {
            pwd: None,
            timeout: Timeout::default(),
            env: HashMap::new(),
            commands: Vec::new(),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeout {
    pub absolute: Option<u64>,
    pub relative: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutateLanguage {
    pub files: FileFilter,
    pub mutants: MutantFilter,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MutantFilter {
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
            VcsConfig::Jj {
                rev: "trunk()".into()
            }
        );
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);

        assert_eq!(config.dirs.working, "/tmp/bough/work");
        assert_eq!(config.dirs.state, "/tmp/bough/state");
        assert_eq!(config.dirs.logs, "/tmp/bough/logs");

        let vitest = &config.runners["vitest"];
        assert_eq!(vitest.treat_timeouts_as, Outcome::Missed);

        let init = vitest.init.as_ref().unwrap();
        assert_eq!(init.pwd.as_deref(), Some("./examples/vitest"));
        assert_eq!(init.commands, vec!["npm install"]);

        let test = vitest.test.as_ref().unwrap();
        assert_eq!(test.timeout.absolute, Some(30));
        assert_eq!(test.timeout.relative, Some(3));
        assert_eq!(test.env["NODE_ENV"], "production");
        assert_eq!(test.commands, vec!["npx run build", "npx run test"]);

        let js_mutate = &vitest.mutate["js"];
        assert_eq!(js_mutate.files.include, vec!["**/*.js", "**/*.jsx"]);
        assert_eq!(js_mutate.files.exclude, vec!["**/*__mocks__*"]);
        assert_eq!(js_mutate.mutants.skip.len(), 2);

        let cargo = &config.runners["cargo"];
        assert_eq!(cargo.pwd.as_deref(), Some("./examples/cargo"));
        let cargo_test = cargo.test.as_ref().unwrap();
        assert_eq!(cargo_test.commands, vec!["cargo test"]);
    }

    #[test]
    fn deserialize_minimal_config() {
        let toml_str = include_str!("minimal.config.toml");
        let config: Config = toml::from_str(toml_str).expect("failed to parse minimal config");

        assert_eq!(config.vcs, VcsConfig::None);
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);
        assert_eq!(config.dirs, Dirs::default());

        let vitest = &config.runners["vitest"];
        assert_eq!(vitest.treat_timeouts_as, Outcome::Missed);
        assert!(vitest.init.is_none());
        assert!(vitest.reset.is_none());

        let test = vitest.test.as_ref().unwrap();
        assert_eq!(test.commands, vec!["npx vitest run"]);
        assert!(test.pwd.is_none());
        assert!(test.env.is_empty());
        assert_eq!(test.timeout, Timeout::default());

        let ts_mutate = &vitest.mutate["ts"];
        assert_eq!(ts_mutate.files.include, vec!["src/**/*.ts"]);
        assert!(ts_mutate.files.exclude.is_empty());
        assert!(ts_mutate.mutants.skip.is_empty());
    }

    #[test]
    fn defaults_are_sane() {
        let config: Config = toml::from_str("").expect("empty config should parse");
        assert_eq!(config.vcs, VcsConfig::None);
        assert_eq!(config.parallelism, 1);
        assert_eq!(config.ordering, Ordering::Random);
        assert_eq!(config.dirs, Dirs::default());
        assert!(config.runners.is_empty());
    }
}

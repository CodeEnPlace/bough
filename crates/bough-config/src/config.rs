use std::collections::HashMap;
use std::path::PathBuf;

use facet::Facet;
use tracing::debug;

use crate::{
    Factor, FindMutationsConfig, LanguageConfig, PhaseConfig, PhaseOverrides, SessionConfig,
    TestPhaseConfig,
};

#[derive(Facet, Debug, Clone)]
pub struct Config {
    #[facet(default = 1)]
    pub workers: u64,

    #[facet(default = 1)]
    pub threads: u64,

    pub base_root_dir: String,

    pub include: Vec<String>,

    pub exclude: Vec<String>,

    pub lang: HashMap<bough_core::LanguageId, LanguageConfig>,

    #[facet(flatten, default)]
    pub phase_defaults: PhaseOverrides,

    #[facet(default)]
    pub test: Option<TestPhaseConfig>,

    #[facet(default)]
    pub init: Option<PhaseConfig>,

    #[facet(default)]
    pub reset: Option<PhaseConfig>,

    #[facet(default)]
    pub find: FindMutationsConfig,
}

trait HasPhaseOverrides {
    fn phase_overrides(&self) -> &PhaseOverrides;
}

impl HasPhaseOverrides for TestPhaseConfig {
    fn phase_overrides(&self) -> &PhaseOverrides {
        &self.overrides
    }
}

impl HasPhaseOverrides for PhaseConfig {
    fn phase_overrides(&self) -> &PhaseOverrides {
        &self.overrides
    }
}

impl Config {
    fn phase_overrides<T: HasPhaseOverrides>(&self, phase: &Option<T>) -> PhaseOverrides {
        phase
            .as_ref()
            .map(|p| p.phase_overrides().clone())
            .unwrap_or_default()
    }

    pub fn phase_timeout_overrides(&self) -> Vec<(&str, &PhaseOverrides)> {
        let mut out = Vec::new();
        if let Some(ref t) = self.test {
            out.push(("test", &t.overrides));
        }
        if let Some(ref i) = self.init {
            out.push(("init", &i.overrides));
        }
        if let Some(ref r) = self.reset {
            out.push(("reset", &r.overrides));
        }
        out
    }
}

const VCS_DIRS: &[&str] = &[".git", ".jj", ".hg", ".svn"];

pub fn collect_vcs_ignore_globs(root: &std::path::Path) -> Vec<String> {
    let mut globs = Vec::new();
    let mut dir = Some(root.to_path_buf());
    while let Some(d) = dir {
        let gitignore = d.join(".gitignore");
        if let Ok(content) = std::fs::read_to_string(&gitignore) {
            debug!(path = %gitignore.display(), "reading vcs ignore file");
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                    continue;
                }
                let pattern = if trimmed.starts_with('/') {
                    trimmed[1..].to_string()
                } else if trimmed.contains('/') {
                    trimmed.to_string()
                } else if trimmed.starts_with("**/") {
                    trimmed.to_string()
                } else {
                    format!("**/{trimmed}")
                };
                globs.push(pattern);
            }
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    globs
}

pub fn collect_vcs_dir_globs(root: &std::path::Path) -> Vec<String> {
    VCS_DIRS
        .iter()
        .filter(|d| root.join(d).is_dir())
        .map(|d| format!("{d}/**"))
        .collect()
}

impl SessionConfig for Config {
    fn get_workers_count(&self) -> u64 {
        self.workers
    }

    fn get_bough_state_dir(&self) -> PathBuf {
        self.get_base_root_path().join(".bough")
    }

    fn get_base_root_path(&self) -> PathBuf {
        PathBuf::from(&self.base_root_dir)
    }

    fn get_base_include_globs(&self) -> impl Iterator<Item = String> {
        self.include.clone().into_iter()
    }

    fn get_base_exclude_globs(&self) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs_ignore = collect_vcs_ignore_globs(&root);
        let vcs_dirs = collect_vcs_dir_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude
            .clone()
            .into_iter()
            .chain(vcs_ignore)
            .chain(vcs_dirs)
            .chain(std::iter::once(bough_glob))
    }

    fn get_test_cmd(&self) -> String {
        self.test
            .as_ref()
            .expect("test.cmd is required")
            .cmd
            .clone()
    }

    fn get_test_pwd(&self) -> PathBuf {
        self.phase_overrides(&self.test)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_test_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.test)
            .resolve_env(&self.phase_defaults)
    }

    fn get_test_timeout(&self, reference: Option<chrono::Duration>) -> chrono::Duration {
        let overrides = self.phase_overrides(&self.test);
        overrides.resolve_timeout(&self.phase_defaults, reference)
    }

    fn get_init_cmd(&self) -> Option<String> {
        self.init.as_ref().and_then(|i| i.cmd.clone())
    }

    fn get_init_pwd(&self) -> PathBuf {
        self.phase_overrides(&self.init)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_init_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.init)
            .resolve_env(&self.phase_defaults)
    }

    fn get_init_timeout(&self, reference: Option<chrono::Duration>) -> chrono::Duration {
        let overrides = self.phase_overrides(&self.init);
        overrides.resolve_timeout(&self.phase_defaults, reference)
    }

    fn get_reset_cmd(&self) -> Option<String> {
        self.reset.as_ref().and_then(|r| r.cmd.clone())
    }

    fn get_reset_pwd(&self) -> PathBuf {
        self.phase_overrides(&self.reset)
            .resolve_pwd(&self.phase_defaults)
    }

    fn get_reset_env(&self) -> HashMap<String, String> {
        self.phase_overrides(&self.reset)
            .resolve_env(&self.phase_defaults)
    }

    fn get_reset_timeout(&self, reference: Option<chrono::Duration>) -> chrono::Duration {
        let overrides = self.phase_overrides(&self.reset);
        overrides.resolve_timeout(&self.phase_defaults, reference)
    }

    fn get_find_number(&self) -> usize {
        self.find.number
    }

    fn get_find_number_per_file(&self) -> usize {
        self.find.number_per_file
    }

    fn get_find_factors(&self) -> Vec<Factor> {
        self.find.factors.clone()
    }

    fn get_langs(&self) -> impl Iterator<Item = bough_core::LanguageId> {
        self.lang.keys().copied().collect::<Vec<_>>().into_iter()
    }

    fn get_lang_include_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        self.lang
            .get(&language_id)
            .map(|c| c.include.clone())
            .unwrap_or_default()
            .into_iter()
    }

    fn get_lang_exclude_globs(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        let root = self.get_base_root_path();
        let vcs_ignore = collect_vcs_ignore_globs(&root);
        let vcs_dirs = collect_vcs_dir_globs(&root);
        let bough_dir = self.get_bough_state_dir();
        let bough_glob = bough_dir
            .strip_prefix(&root)
            .map(|rel| format!("{}/**", rel.display()))
            .unwrap_or_else(|_| format!("{}/**", bough_dir.display()));
        self.exclude
            .iter()
            .cloned()
            .chain(vcs_ignore)
            .chain(vcs_dirs)
            .chain(std::iter::once(bough_glob))
            .chain(
                self.lang
                    .get(&language_id)
                    .map(|c| c.exclude.clone())
                    .unwrap_or_default(),
            )
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn get_lang_skip_queries(
        &self,
        language_id: bough_core::LanguageId,
    ) -> impl Iterator<Item = String> {
        self.lang
            .get(&language_id)
            .and_then(|c| c.skip.as_ref())
            .map(|s| s.query.clone())
            .unwrap_or_default()
            .into_iter()
    }
}

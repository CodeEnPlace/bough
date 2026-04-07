use std::collections::HashMap;
use std::path::PathBuf;

use bough_core::LanguageId;
use chrono::Duration;

use crate::factor::Factor;

pub trait SessionConfig {
    fn get_workers_count(&self) -> u64;

    fn get_bough_state_dir(&self) -> PathBuf;
    fn get_base_root_path(&self) -> PathBuf;
    fn get_base_include_globs(&self) -> impl Iterator<Item = String>;
    fn get_base_exclude_globs(&self) -> impl Iterator<Item = String>;

    fn get_langs(&self) -> impl Iterator<Item = LanguageId>;
    fn get_lang_include_globs(&self, language_id: LanguageId) -> impl Iterator<Item = String>;
    fn get_lang_exclude_globs(&self, language_id: LanguageId) -> impl Iterator<Item = String>;
    fn get_lang_skip_queries(&self, language_id: LanguageId) -> impl Iterator<Item = String>;

    fn get_test_cmd(&self) -> String;
    fn get_test_pwd(&self) -> PathBuf;
    fn get_test_env(&self) -> HashMap<String, String>;
    fn get_test_timeout(&self, reference: Option<Duration>) -> Duration;

    fn get_init_cmd(&self) -> Option<String>;
    fn get_init_pwd(&self) -> PathBuf;
    fn get_init_env(&self) -> HashMap<String, String>;
    fn get_init_timeout(&self, reference: Option<Duration>) -> Duration;

    fn get_reset_cmd(&self) -> Option<String>;
    fn get_reset_pwd(&self) -> PathBuf;
    fn get_reset_env(&self) -> HashMap<String, String>;
    fn get_reset_timeout(&self, reference: Option<Duration>) -> Duration;

    fn get_find_number(&self) -> usize;
    fn get_find_number_per_file(&self) -> usize;
    fn get_find_factors(&self) -> Vec<Factor>;
}

use crate::Hash;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    Plain,
    Pretty,
    Json,
    Markdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

pub trait Render {
    fn render(&self, style: &Style, no_color: bool, depth: u8);
}

pub fn color(code: &str, text: &str, no_color: bool) -> String {
    if no_color {
        text.to_string()
    } else {
        format!("{code}{text}\x1b[0m")
    }
}

pub fn hashed_path(dir: &PathBuf, content: &str, label: &str) -> PathBuf {
    dir.join(format!("{}.{label}.json", Hash::of(content)))
}

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
    fn render_json(&self) -> String;
    fn render_pretty(&self, depth: u8) -> String;
    fn render_markdown(&self, depth: u8) -> String;

    fn render(&self, style: &Style, no_color: bool, depth: u8) {
        let output = match style {
            Style::Json => self.render_json(),
            Style::Markdown => self.render_markdown(depth),
            Style::Pretty if no_color => strip_ansi(&self.render_pretty(depth)),
            Style::Pretty => self.render_pretty(depth),
            Style::Plain => strip_ansi(&self.render_pretty(depth)),
        };
        print!("{output}");
    }
}

pub fn color(code: &str, text: &str) -> String {
    format!("{code}{text}\x1b[0m")
}

pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

pub fn hashed_path(dir: &PathBuf, content: &str, label: &str) -> PathBuf {
    dir.join(format!("{}.{label}.json", Hash::of(content)))
}

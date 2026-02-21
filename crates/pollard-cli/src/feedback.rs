use pollard_core::{Hash, MutationKind};
use serde::Serialize;
use similar::{ChangeTag, TextDiff};
use std::fmt::Write;
use std::path::PathBuf;

#[derive(Clone, clap::ValueEnum)]
pub enum DiffStyle {
    Unified,
    SideBySide,
}

#[derive(Clone, clap::ValueEnum)]
pub enum Style {
    Plain,
    Pretty,
    Json,
}

pub trait RenderOutput {
    fn render(&self, style: &Style, no_color: bool);
}

#[derive(Serialize)]
pub struct MutationRecord {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
    pub kind: MutationKind,
    pub start_line: usize,
    pub start_char: usize,
    pub end_line: usize,
    pub end_char: usize,
    pub original: String,
    pub replacement: String,
}

impl RenderOutput for MutationRecord {
    fn render(&self, style: &Style, no_color: bool) {
        match style {
            Style::Json => {
                println!("{}", serde_json::to_string(self).expect("failed to serialize"));
            }
            Style::Plain | Style::Pretty if no_color => {
                println!(
                    "{} {:?} {}:{}-{}:{} {} -> {}",
                    self.mutated_hash,
                    self.kind,
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                    self.original,
                    self.replacement,
                );
            }
            Style::Plain => {
                println!(
                    "{} {:?} {}:{}-{}:{} {} -> {}",
                    self.mutated_hash,
                    self.kind,
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                    self.original,
                    self.replacement,
                );
            }
            Style::Pretty => {
                println!(
                    "\x1b[33m{}\x1b[0m \x1b[1m{:?}\x1b[0m \x1b[36m{}:{}-{}:{}\x1b[0m \x1b[31m{}\x1b[0m -> \x1b[32m{}\x1b[0m",
                    self.mutated_hash,
                    self.kind,
                    self.start_line + 1,
                    self.start_char + 1,
                    self.end_line + 1,
                    self.end_char + 1,
                    self.original,
                    self.replacement,
                );
            }
        }
    }
}

pub struct DiffRecord {
    pub old: String,
    pub new: String,
    pub path: String,
    pub diff_style: DiffStyle,
}

impl RenderOutput for DiffRecord {
    fn render(&self, _style: &Style, no_color: bool) {
        match self.diff_style {
            DiffStyle::Unified => render_unified(&self.old, &self.new, &self.path, no_color),
            DiffStyle::SideBySide => render_side_by_side(&self.old, &self.new, &self.path, no_color),
        }
    }
}

fn color(code: &str, text: &str, no_color: bool) -> String {
    if no_color {
        text.to_string()
    } else {
        format!("{code}{text}\x1b[0m")
    }
}

fn render_unified(old: &str, new: &str, path: &str, no_color: bool) {
    let diff = TextDiff::from_lines(old, new);
    println!("{}", color("\x1b[1m", &format!("--- {path}"), no_color));
    println!("{}", color("\x1b[1m", &format!("+++ {path} (mutated)"), no_color));
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        let mut buf = String::new();
        write!(&mut buf, "{hunk}").unwrap();
        for line in buf.lines() {
            if line.starts_with("@@") {
                println!("{}", color("\x1b[36m", line, no_color));
            } else if line.starts_with('-') {
                println!("{}", color("\x1b[31m", line, no_color));
            } else if line.starts_with('+') {
                println!("{}", color("\x1b[32m", line, no_color));
            } else {
                println!(" {line}");
            }
        }
    }
}

fn render_side_by_side(old: &str, new: &str, path: &str, no_color: bool) {
    let diff = TextDiff::from_lines(old, new);
    let width = terminal_width() / 2 - 3;

    println!(
        "{} │ {}",
        color("\x1b[1m", &pad(path, width), no_color),
        color("\x1b[1m", &format!("{path} (mutated)"), no_color),
    );
    println!("{}─┼─{}", "─".repeat(width), "─".repeat(width));

    for change in diff.iter_all_changes() {
        let text = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Equal => {
                println!("{} │ {}", pad(text, width), text);
            }
            ChangeTag::Delete => {
                println!("{} │", color("\x1b[31m", &pad(text, width), no_color));
            }
            ChangeTag::Insert => {
                println!("{} │ {}", pad("", width), color("\x1b[32m", text, no_color));
            }
        }
    }
}

fn pad(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        format!("{s:<width$}")
    }
}

#[derive(Serialize)]
pub struct ApplyRecord {
    pub source_path: PathBuf,
    pub source_hash: Hash,
    pub mutated_hash: Hash,
}

impl RenderOutput for ApplyRecord {
    fn render(&self, style: &Style, no_color: bool) {
        match style {
            Style::Json => {
                println!("{}", serde_json::to_string(self).expect("failed to serialize"));
            }
            Style::Pretty => {
                println!(
                    "Applied mutation {} to {}",
                    color("\x1b[33m", &self.mutated_hash.to_string(), no_color),
                    color("\x1b[36m", &self.source_path.display().to_string(), no_color),
                );
            }
            Style::Plain => {
                println!(
                    "Applied mutation {} to {}",
                    self.mutated_hash,
                    self.source_path.display(),
                );
            }
        }
    }
}

fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120)
}

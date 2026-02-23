use crate::io::{Action, DiffStyle, Render};
use crate::mutate::find_mutated;
use bough_core::config::LanguageId;
use bough_core::io::color;
use bough_core::{Hash, SourceFile};
use similar::{ChangeTag, TextDiff};
use std::fmt::Write;
use std::path::Path;

pub fn run(
    language: &LanguageId,
    input: &Path,
    hash: &Hash,
    diff_style: DiffStyle,
) -> (Vec<Action>, ViewReport) {
    let file = SourceFile::read(input).expect("failed to read input file");
    let mutated = find_mutated(language, &file, hash);

    let report = ViewReport {
        old: file.content().to_string(),
        new: mutated.content().to_string(),
        path: file.path().display().to_string(),
        diff_style,
    };

    (vec![], report)
}

pub struct ViewReport {
    pub old: String,
    pub new: String,
    pub path: String,
    pub diff_style: DiffStyle,
}

impl Render for ViewReport {
    fn render_json(&self) -> String {
        serde_json::to_string(&serde_json::json!({
            "path": self.path,
            "old": self.old,
            "new": self.new,
        }))
        .expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        match self.diff_style {
            DiffStyle::Unified => format_unified(&self.old, &self.new, &self.path),
            DiffStyle::SideBySide => format_side_by_side(&self.old, &self.new, &self.path),
        }
    }

    fn render_markdown(&self, depth: u8) -> String {
        self.render_pretty(depth)
    }
}

fn format_unified(old: &str, new: &str, path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::new();
    out.push_str(&color("\x1b[1m", &format!("--- {path}")));
    out.push('\n');
    out.push_str(&color("\x1b[1m", &format!("+++ {path} (mutated)")));
    out.push('\n');
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        let mut buf = String::new();
        write!(&mut buf, "{hunk}").unwrap();
        for line in buf.lines() {
            if line.starts_with("@@") {
                out.push_str(&color("\x1b[36m", line));
            } else if line.starts_with('-') {
                out.push_str(&color("\x1b[31m", line));
            } else if line.starts_with('+') {
                out.push_str(&color("\x1b[32m", line));
            } else {
                out.push(' ');
                out.push_str(line);
            }
            out.push('\n');
        }
    }
    out
}

fn format_side_by_side(old: &str, new: &str, path: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let width = terminal_width() / 2 - 3;
    let mut out = String::new();

    out.push_str(&format!(
        "{} │ {}\n",
        color("\x1b[1m", &pad(path, width)),
        color("\x1b[1m", &format!("{path} (mutated)")),
    ));
    out.push_str(&format!("{}─┼─{}\n", "─".repeat(width), "─".repeat(width)));

    for change in diff.iter_all_changes() {
        let text = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Equal => {
                out.push_str(&format!("{} │ {}\n", pad(text, width), text));
            }
            ChangeTag::Delete => {
                out.push_str(&format!("{} │\n", color("\x1b[31m", &pad(text, width))));
            }
            ChangeTag::Insert => {
                out.push_str(&format!(
                    "{} │ {}\n",
                    pad("", width),
                    color("\x1b[32m", text)
                ));
            }
        }
    }
    out
}

fn pad(s: &str, width: usize) -> String {
    if s.len() >= width {
        s[..width].to_string()
    } else {
        format!("{s:<width$}")
    }
}

fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120)
}

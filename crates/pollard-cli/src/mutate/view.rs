use crate::io::{Action, DiffStyle, Report, Style, hashed_path};
use std::path::PathBuf;
use crate::mutate::find_mutated;
use pollard_core::config::LanguageId;
use pollard_core::{Hash, SourceFile};
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

impl Report for ViewReport {
    fn get_dir(&self, session: &crate::session::Session) -> PathBuf {
        session.report_dir.join("mutate").join("view")
    }

    fn make_path(&self, session: &crate::session::Session) -> PathBuf {
        let content = format!("{}:{}:{}", self.path, self.old, self.new);
        hashed_path(&self.get_dir(session), &content, "diff")
    }

    fn render(&self, _style: &Style, no_color: bool, _depth: u8) {
        match self.diff_style {
            DiffStyle::Unified => render_unified(&self.old, &self.new, &self.path, no_color),
            DiffStyle::SideBySide => {
                render_side_by_side(&self.old, &self.new, &self.path, no_color)
            }
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
    println!(
        "{}",
        color("\x1b[1m", &format!("+++ {path} (mutated)"), no_color)
    );
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

fn terminal_width() -> usize {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 as usize)
        .unwrap_or(120)
}

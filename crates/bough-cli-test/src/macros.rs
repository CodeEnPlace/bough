use std::path::{Path, PathBuf};

pub struct TestPlan {
    config: Option<String>,
    files: Vec<(String, String)>,
}

impl TestPlan {
    pub fn new() -> Self {
        Self {
            config: None,
            files: vec![],
        }
    }

    pub fn config(mut self, content: &str) -> Self {
        self.config = Some(content.to_string());
        self
    }

    pub fn file(mut self, path: &str, content: &str) -> Self {
        self.files.push((path.to_string(), content.to_string()));
        self
    }

    pub fn setup(self) -> TestDir {
        let dir = tempfile::tempdir().unwrap();

        if let Some(config) = &self.config {
            std::fs::write(dir.path().join(".bough.config.toml"), config).unwrap();
        }

        for (path, content) in &self.files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(full, content).unwrap();
        }

        std::process::Command::new("jj")
            .args(["git", "init"])
            .current_dir(dir.path())
            .output()
            .ok();

        TestDir { dir }
    }
}

pub struct TestDir {
    dir: tempfile::TempDir,
}

impl TestDir {
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn run_success(&self, cmd_str: &str) -> String {
        let output = self.exec(cmd_str);

        assert!(
            output.status.success(),
            "expected success for: {cmd_str}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        String::from_utf8(output.stdout).unwrap()
    }

    pub fn run_failure(&self, cmd_str: &str) -> String {
        let output = self.exec(cmd_str);

        assert!(
            !output.status.success(),
            "expected failure for: {cmd_str}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );

        String::from_utf8(output.stderr).unwrap()
    }

    fn exec(&self, cmd_str: &str) -> std::process::Output {
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        let (program, args) = parts.split_first().expect("empty command");

        let program = resolve_program(program);

        std::process::Command::new(&program)
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap_or_else(|e| panic!("failed to execute '{cmd_str}': {e}"))
    }
}

fn resolve_program(name: &str) -> PathBuf {
    if name == "bough" {
        #[allow(deprecated)]
        assert_cmd::cargo::cargo_bin("bough")
    } else {
        PathBuf::from(name)
    }
}

/// Try to match a single line against needles (fixed segments between captures).
///
/// `needles` has length = num_captures + 1. The first needle must be at the
/// start of the line, the last must be at the end, and captures fill the gaps.
pub fn match_line(line: &str, needles: &[&str]) -> Option<Vec<String>> {
    let mut captures = Vec::with_capacity(needles.len().saturating_sub(1));
    let mut cursor = 0;

    for (i, needle) in needles.iter().enumerate() {
        if i == 0 {
            if !needle.is_empty() {
                if !line.starts_with(needle) {
                    return None;
                }
                cursor = needle.len();
            }
        } else if needle.is_empty() {
            captures.push(line[cursor..].to_string());
        } else {
            let pos = line[cursor..].find(needle)?;
            captures.push(line[cursor..cursor + pos].to_string());
            cursor += pos + needle.len();
        }
    }

    if !needles.last().unwrap().is_empty() && cursor != line.len() {
        return None;
    }

    Some(captures)
}

/// Search forward through `lines` for one that matches `needles`.
/// Returns (line_index, captured_values).
pub fn find_line(lines: &[&str], needles: &[&str]) -> Option<(usize, Vec<String>)> {
    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = match_line(line.trim(), needles) {
            return Some((i, caps));
        }
    }
    None
}

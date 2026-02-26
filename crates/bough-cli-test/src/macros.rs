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
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        self.dir.path()
    }
}

pub fn exec_cmd(dir: &Path, cmd_str: &str) -> std::process::Output {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    let (program, args) = parts.split_first().expect("empty command");

    let program = resolve_program(program);

    std::process::Command::new(&program)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute '{cmd_str}': {e}"))
}

fn resolve_program(name: &str) -> PathBuf {
    if name == "bough" {
        #[allow(deprecated)]
        assert_cmd::cargo::cargo_bin("bough")
    } else {
        PathBuf::from(name)
    }
}

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

pub fn find_unmatched_line(
    lines: &[&str],
    matched: &[bool],
    needles: &[&str],
) -> Option<(usize, Vec<String>)> {
    for (i, line) in lines.iter().enumerate() {
        if matched[i] {
            continue;
        }
        if let Some(caps) = match_line(line.trim(), needles) {
            return Some((i, caps));
        }
    }
    None
}

use std::collections::HashMap;
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

        TestDir {
            dir,
            captures: HashMap::new(),
        }
    }
}

pub struct TestDir {
    dir: tempfile::TempDir,
    captures: HashMap<String, String>,
}

impl TestDir {
    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn get(&self, name: &str) -> &str {
        self.captures
            .get(name)
            .unwrap_or_else(|| panic!("no capture named '{name}'"))
    }

    pub fn run_success(&mut self, cmd_str: &str, pattern: &str) {
        let output = self.exec(cmd_str);

        assert!(
            output.status.success(),
            "expected success for: {cmd_str}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).unwrap();
        self.match_pattern(&stdout, pattern);
    }

    pub fn run_failure(&mut self, cmd_str: &str, pattern: &str) {
        let output = self.exec(cmd_str);

        assert!(
            !output.status.success(),
            "expected failure for: {cmd_str}\nstdout: {}",
            String::from_utf8_lossy(&output.stdout)
        );

        let stderr = String::from_utf8(output.stderr).unwrap();
        self.match_pattern(&stderr, pattern);
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

    fn match_pattern(&mut self, text: &str, pattern: &str) {
        let pattern = pattern.trim();
        let text = text.trim();

        let mut regex_str = String::from("(?s)^");
        let mut capture_names = Vec::new();
        let pat_bytes = pattern.as_bytes();
        let mut i = 0;

        while i < pat_bytes.len() {
            if i + 2 < pat_bytes.len()
                && pat_bytes[i] == b'{'
                && (pat_bytes[i + 1] == b'!' || pat_bytes[i + 1] == b'?')
            {
                let mode = pat_bytes[i + 1];
                let rest = &pattern[i + 2..];
                let close = rest
                    .find('}')
                    .unwrap_or_else(|| panic!("unclosed capture in pattern: {pattern}"));
                let name = &pattern[i + 2..i + 2 + close];

                if mode == b'!' {
                    regex_str.push_str(&format!("(?P<{name}>\\S+)"));
                    capture_names.push(name.to_string());
                } else {
                    let val = self
                        .captures
                        .get(name)
                        .unwrap_or_else(|| panic!("no previous capture named '{name}'"));
                    regex_str.push_str(&regex::escape(val));
                }

                i += 2 + close + 1;
            } else {
                regex_str.push_str(&regex::escape(&pattern[i..i + 1]));
                i += 1;
            }
        }

        regex_str.push('$');

        let re = regex::Regex::new(&regex_str).unwrap();
        let caps = re.captures(text).unwrap_or_else(|| {
            panic!("pattern did not match\npattern: {pattern}\nregex:   {regex_str}\ntext:    {text}");
        });

        for name in &capture_names {
            if let Some(m) = caps.name(name) {
                self.captures
                    .insert(name.clone(), m.as_str().to_string());
            }
        }
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

#[macro_export]
macro_rules! cmd {
    ($dir:expr, $cmd:expr, $pattern:expr) => {
        $dir.run_success($cmd, $pattern)
    };
    ($dir:expr, $cmd:expr, $stdout:expr, $stderr:expr) => {
        $dir.run_failure($cmd, $stderr)
    };
}

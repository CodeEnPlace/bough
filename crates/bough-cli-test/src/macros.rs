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

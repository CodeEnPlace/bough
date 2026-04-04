use std::path::{Path, PathBuf};
use std::process::Command;

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // skip until we hit a letter (the terminator of the escape sequence)
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// Result of running a bough command.
pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    /// Return stderr with the fixture's temp dir path replaced by `<TMP>`
    /// and ANSI escape codes stripped.
    pub fn redacted_stderr(&self, fixture: &Fixture) -> String {
        let path_str = fixture.path().to_string_lossy();
        strip_ansi(&self.stderr.replace(path_str.as_ref(), "<TMP>"))
    }

    /// Return stdout with the fixture's temp dir path replaced by `<TMP>`
    /// and ANSI escape codes stripped.
    pub fn redacted_stdout(&self, fixture: &Fixture) -> String {
        let path_str = fixture.path().to_string_lossy();
        strip_ansi(&self.stdout.replace(path_str.as_ref(), "<TMP>"))
    }
}

/// Pending file to be written when the fixture is built.
struct PendingFile {
    path: PathBuf,
    content: String,
}

/// Builder for a test fixture directory.
pub struct FixtureBuilder {
    files: Vec<PendingFile>,
}

/// A built test fixture backed by a temporary directory.
pub struct Fixture {
    dir: tempfile::TempDir,
}

impl FixtureBuilder {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn with_file(mut self, path: impl Into<PathBuf>, content: impl Into<String>) -> Self {
        self.files.push(PendingFile {
            path: path.into(),
            content: content.into(),
        });
        self
    }

    pub fn build(self) -> Fixture {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        for file in &self.files {
            let full_path = dir.path().join(&file.path);
            if let Some(parent) = full_path.parent() {
                std::fs::create_dir_all(parent).expect("failed to create parent dirs");
            }
            std::fs::write(&full_path, &file.content).expect("failed to write file");
        }
        Fixture { dir }
    }
}

impl Fixture {
    pub fn new() -> FixtureBuilder {
        FixtureBuilder::new()
    }

    /// Run a bough command in this fixture's directory.
    /// `args` is a string of space-separated arguments (e.g. "show config -f json").
    pub fn run(&self, args: &str) -> Output {
        let bough = env!("BOUGH_BIN");
        let args: Vec<&str> = args.split_whitespace().collect();
        let output = Command::new(bough)
            .args(&args)
            .current_dir(self.dir.path())
            .env("NO_COLOR", "1")
            .output()
            .unwrap_or_else(|e| panic!("failed to execute bough at {bough}: {e}"));

        Output {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8(output.stdout).expect("stdout not utf8"),
            stderr: String::from_utf8(output.stderr).expect("stderr not utf8"),
        }
    }

    /// Return the path to the fixture's temporary directory.
    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

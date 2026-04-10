use std::path::Path;
use std::process::Command;

pub struct Output {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Output {
    pub fn redacted_stderr(&self, fixture: &Fixture) -> String {
        redact(&self.stderr, fixture)
    }

    pub fn redacted_stdout(&self, fixture: &Fixture) -> String {
        redact(&self.stdout, fixture)
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
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

fn redact(s: &str, fixture: &Fixture) -> String {
    let stripped = strip_ansi(s);
    // On macOS, /var is a symlink to /private/var — canonicalize to match bough output.
    // On Windows, canonicalize expands 8.3 short names (RUNNER~1 → runneradmin) which
    // diverges from what bough sees via current_dir(), so we skip it.
    #[cfg(unix)]
    let base = fixture.path().canonicalize().unwrap_or_else(|_| fixture.path().to_path_buf());
    #[cfg(not(unix))]
    let base = fixture.path().to_path_buf();
    let fixture_path = base.to_string_lossy().into_owned();
    // Generate variants to handle OS path separator differences in output
    let tmp_json = fixture_path.replace('\\', "\\\\");
    let tmp_fwd = fixture_path.replace('\\', "/");
    let mut result = String::with_capacity(stripped.len());
    let mut in_file_block = false;
    for line in stripped.lines() {
        if line.contains("├─ file:") {
            in_file_block = true;
            result.push_str("├─ file: <CONFIG_SEARCH_PATHS>\n");
            continue;
        }
        if in_file_block {
            if !line.starts_with("│") {
                in_file_block = false;
            } else {
                continue;
            }
        }
        let redacted = line
            .replace(tmp_json.as_str(), "<TMP>")
            .replace(tmp_fwd.as_str(), "<TMP>")
            .replace(fixture_path.as_str(), "<TMP>")
            .replace("<TMP>\\\\", "<TMP>/")
            .replace("<TMP>\\", "<TMP>/");
        result.push_str(&redacted);
        result.push('\n');
    }
    if !s.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }
    result
}

/// Pending file to be written when the fixture is built.
struct PendingFile {
    path: std::path::PathBuf,
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

impl Default for FixtureBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FixtureBuilder {
    pub fn new() -> Self {
        Self { files: Vec::new() }
    }

    pub fn with_file(mut self, path: impl Into<std::path::PathBuf>, content: impl Into<String>) -> Self {
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
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> FixtureBuilder {
        FixtureBuilder::new()
    }

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

    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}

use crate::languages::LanguageId;
use bough_typed_hash::HashInto;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// core[impl source]
pub struct SourceDir {
    path: PathBuf,
    files_config: crate::config::FileSourceConfig,
}

#[derive(Debug)]
pub enum Error {
    NotADirectory(PathBuf),
    Glob(glob::PatternError),
    GlobIter(glob::GlobError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotADirectory(p) => write!(f, "source dir is not a directory: {}", p.display()),
            Error::Glob(e) => write!(f, "invalid glob pattern: {e}"),
            Error::GlobIter(e) => write!(f, "glob iteration error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl SourceDir {
    // core[impl source.new]
    pub fn new(config: &crate::config::Config) -> Result<Self, Error> {
        let path = config.source_dir.clone();
        if !path.is_dir() {
            return Err(Error::NotADirectory(path));
        }
        Ok(Self {
            path,
            files_config: config.files.clone(),
        })
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self {
            path,
            files_config: crate::config::FileSourceConfig::default(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // core[impl source.files.iter]
    pub fn all_files(&self) -> Result<Vec<SourceFile>, Error> {
        let mut files = Vec::new();
        let exclude_patterns: Vec<glob::Pattern> = self
            .files_config
            .exclude
            .iter()
            .map(|e| glob::Pattern::new(&self.path.join(e).to_string_lossy()))
            .collect::<Result<_, _>>()
            .map_err(Error::Glob)?;

        // core[impl source.files.include]
        for include in &self.files_config.include {
            let pattern = self.path.join(include).to_string_lossy().into_owned();
            for entry in glob::glob(&pattern).map_err(Error::Glob)? {
                let path = entry.map_err(Error::GlobIter)?;
                if path.is_file()
                    && !exclude_patterns.iter().any(|p| p.matches_path(&path))
                {
                    files.push(SourceFile::new(path));
                }
            }
        }
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ConfigBuilder;

    const BASE_CONFIG: &str = r#"
[test]
command = "npx vitest run"

[mutate.ts]
files.include = ["src/**/*.ts"]
"#;

    fn build_config(source_dir: PathBuf) -> crate::config::Config {
        let v: toml::Value = toml::from_str(BASE_CONFIG).unwrap();
        ConfigBuilder::new(source_dir)
            .from_value(v)
            .build()
            .unwrap()
    }

    // core[verify source]
    #[test]
    fn source_dir_holds_directory_path() {
        let dir = PathBuf::from("/tmp/test-source");
        let sd = SourceDir { path: dir.clone(), files_config: crate::config::FileSourceConfig::default() };
        assert_eq!(sd.path(), Path::new("/tmp/test-source"));
    }

    // core[verify source.new]
    #[test]
    fn new_from_valid_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = build_config(tmp.path().to_path_buf());
        let sd = SourceDir::new(&config).unwrap();
        assert_eq!(sd.path(), tmp.path());
    }

    // core[verify source.new]
    #[test]
    fn new_errors_if_source_dir_missing() {
        let config = build_config(PathBuf::from("/nonexistent/path/abc123"));
        let result = SourceDir::new(&config);
        assert!(result.is_err());
    }

    fn copy_dir_recursive(src: &Path, dst: &Path) {
        std::fs::create_dir_all(dst).unwrap();
        for entry in std::fs::read_dir(src).unwrap() {
            let entry = entry.unwrap();
            let dest = dst.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                if entry.file_name() == "node_modules" {
                    continue;
                }
                copy_dir_recursive(&entry.path(), &dest);
            } else {
                std::fs::copy(entry.path(), dest).unwrap();
            }
        }
    }

    fn setup_vitest_js() -> tempfile::TempDir {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/vitest-js");
        copy_dir_recursive(&src, tmp.path());
        tmp
    }

    const FILES_CONFIG: &str = r#"
[test]
command = "npx vitest run"

[files]
include = ["src/**/*.js"]
exclude = ["src/**/*.test.js"]

[mutate.js]
files.include = ["src/**/*.js"]
"#;

    fn build_files_config(source_dir: PathBuf) -> crate::config::Config {
        let v: toml::Value = toml::from_str(FILES_CONFIG).unwrap();
        ConfigBuilder::new(source_dir)
            .from_value(v)
            .build()
            .unwrap()
    }

    // core[verify source.files.iter]
    #[test]
    fn all_files_returns_matched_files() {
        let tmp = setup_vitest_js();
        let config = build_files_config(tmp.path().to_path_buf());
        let sd = SourceDir::new(&config).unwrap();
        let files = sd.all_files().unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        assert!(paths.iter().any(|p| p.ends_with("index.js")));
        assert!(!paths.iter().any(|p| p.to_string_lossy().contains(".test.")));
    }

    // core[verify source.files.include]
    #[test]
    fn include_glob_matches_files() {
        let tmp = setup_vitest_js();
        let config_str = r#"
[test]
command = "npx vitest run"

[files]
include = ["src/**/*.test.js"]

[mutate.js]
files.include = ["src/**/*.js"]
"#;
        let v: toml::Value = toml::from_str(config_str).unwrap();
        let config = ConfigBuilder::new(tmp.path().to_path_buf())
            .from_value(v)
            .build()
            .unwrap();
        let sd = SourceDir::new(&config).unwrap();
        let files = sd.all_files().unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.clone()).collect();
        assert!(!paths.is_empty());
        assert!(paths.iter().all(|p| p.to_string_lossy().contains(".test.")));
        assert!(!paths.iter().any(|p| p.ends_with("index.js")));
    }

    // core[verify source.files.iter]
    #[test]
    fn all_files_empty_when_no_includes() {
        let tmp = setup_vitest_js();
        let config = build_config(tmp.path().to_path_buf());
        let sd = SourceDir::new(&config).unwrap();
        let files = sd.all_files().unwrap();
        assert!(files.is_empty());
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bough_typed_hash::TypedHashable)]
pub struct SourceFile {
    pub path: PathBuf,
}

impl SourceFile {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bough_typed_hash::TypedHashable)]
pub struct MutationSourceFile {
    pub file: SourceFile,
    pub language: LanguageId,
}

impl MutationSourceFile {
    pub fn new(file: SourceFile, language: LanguageId) -> Self {
        Self { file, language }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, HashInto)]
pub struct Point {
    pub src: SourceFile,
    pub line: usize,
    pub char: usize,
    pub byte: usize,
}

impl Point {
    pub fn from_ts(file: &SourceFile, ts: tree_sitter::Point, byte: usize) -> Self {
        Self {
            src: file.clone(),
            line: ts.row,
            char: ts.column,
            byte,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, HashInto)]
pub struct Span {
    pub start: Point,
    pub end: Point,
}

impl Span {
    pub fn from_node(file: &SourceFile, node: tree_sitter::Node<'_>) -> Self {
        Self {
            start: Point::from_ts(file, node.start_position(), node.start_byte()),
            end: Point::from_ts(file, node.end_position(), node.end_byte()),
        }
    }
}

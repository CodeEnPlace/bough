use crate::languages::LanguageId;
use bough_typed_hash::HashInto;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// core[impl source]
pub struct SourceDir {
    path: PathBuf,
}

#[derive(Debug)]
pub enum Error {
    NotADirectory(PathBuf),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotADirectory(p) => write!(f, "source dir is not a directory: {}", p.display()),
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
        Ok(Self { path })
    }

    pub fn from_path(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
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
        let sd = SourceDir { path: dir.clone() };
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bough_typed_hash::TypedHashable)]
pub struct SourceFile {
    pub path: PathBuf,
    pub language: LanguageId,
}

impl SourceFile {
    pub fn read(path: &Path, language: LanguageId) -> std::io::Result<Self> {
        Ok(Self {
            path: path.to_owned(),
            language,
        })
    }

    pub fn from_content(path: PathBuf, _content: &str, language: LanguageId) -> Self {
        Self { path, language }
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

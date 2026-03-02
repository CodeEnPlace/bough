use crate::languages::LanguageId;
use bough_typed_hash::HashInto;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub struct SourceDir {}

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

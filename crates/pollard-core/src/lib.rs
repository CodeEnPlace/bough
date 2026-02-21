pub mod languages;

use std::path::{Path, PathBuf};
use tree_sitter::Parser;

pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
}

impl SourceFile {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            path: path.to_owned(),
            content,
        })
    }
}

#[derive(Debug, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

pub trait Language {
    type Kind: Into<MutationKind>;

    fn tree_sitter_language() -> tree_sitter::Language;
    fn mutation_kind_for_node(node_kind: &str) -> Option<Self::Kind>;
}

#[derive(Debug, PartialEq)]
pub enum MutationKind {
    StatementBlock,
}

pub struct MutationPoint<'a, L: Language> {
    pub file: &'a SourceFile,
    pub span: Span,
    pub kind: L::Kind,
}

pub fn find_mutation_points<'a, L: Language>(file: &'a SourceFile) -> Vec<MutationPoint<'a, L>> {
    let mut parser = Parser::new();
    parser
        .set_language(&L::tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser
        .parse(&file.content, None)
        .expect("failed to parse source");

    let mut points = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some(kind) = L::mutation_kind_for_node(node.kind()) {
            points.push(MutationPoint {
                file,
                span: Span {
                    start: node.start_byte(),
                    end: node.end_byte(),
                },
                kind,
            });
        }

        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return points;
            }
        }
    }
}

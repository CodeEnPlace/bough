pub mod config;
pub mod io;
pub mod languages;

use bough_sha::{ShaHash, ShaHashable};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ShaHashable)]
pub struct SourceFile {
    pub path: PathBuf,
    pub hash: ShaHash,
}

impl SourceFile {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            path: path.to_owned(),
            hash: content.sha_hash(),
        })
    }

    pub fn from_content(path: PathBuf, content: &str) -> Self {
        Self {
            path,
            hash: content.sha_hash(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ShaHashable)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ShaHashable)]
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, ShaHashable)]
pub enum BinaryOpKind {
    Add,
    Sub,
    Mul,
    Div,
    And,
    Or,
    StrictEq,
    StrictNeq,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

impl BinaryOpKind {
    pub fn label(&self) -> &'static str {
        match self {
            BinaryOpKind::Add => "Add (+)",
            BinaryOpKind::Sub => "Subtract (-)",
            BinaryOpKind::Mul => "Multiply (*)",
            BinaryOpKind::Div => "Divide (/)",
            BinaryOpKind::And => "Logical And (&&)",
            BinaryOpKind::Or => "Logical Or (||)",
            BinaryOpKind::StrictEq => "Strict Equal (===)",
            BinaryOpKind::StrictNeq => "Strict Not Equal (!==)",
            BinaryOpKind::Eq => "Equal (==)",
            BinaryOpKind::Neq => "Not Equal (!=)",
            BinaryOpKind::Lt => "Less Than (<)",
            BinaryOpKind::Lte => "Less Than or Equal (<=)",
            BinaryOpKind::Gt => "Greater Than (>)",
            BinaryOpKind::Gte => "Greater Than or Equal (>=)",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ShaHashable)]
pub enum MutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
}

pub trait Language: Debug {
    type Kind: Debug + Clone + PartialEq + Serialize + for<'de> Deserialize<'de> + ShaHashable + Into<MutationKind>;

    fn code_tag() -> &'static str;
    fn tree_sitter_language() -> tree_sitter::Language;
    fn mutation_kind_for_node(
        node: tree_sitter::Node<'_>,
        content: &[u8],
        file: &SourceFile,
    ) -> Option<(Self::Kind, Span)>;
    fn substitutions_for_kind(kind: &Self::Kind) -> Vec<String>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ShaHashable)]
#[serde(bound = "")]
pub struct Mutant<L: Language> {
    pub src: SourceFile,
    pub span: Span,
    pub kind: L::Kind,
}

impl<L: Language> Clone for Mutant<L> {
    fn clone(&self) -> Self {
        Self {
            src: self.src.clone(),
            span: self.span.clone(),
            kind: self.kind.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ShaHashable)]
#[serde(bound = "")]
pub struct Mutation<L: Language> {
    pub mutant: Mutant<L>,
    pub replacement: String,
}

impl<L: Language> Clone for Mutation<L> {
    fn clone(&self) -> Self {
        Self {
            mutant: self.mutant.clone(),
            replacement: self.replacement.clone(),
        }
    }
}

pub fn find_mutants<L: Language>(file: &SourceFile, content: &str) -> Vec<Mutant<L>> {
    let mut parser = Parser::new();
    parser
        .set_language(&L::tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser
        .parse(content, None)
        .expect("failed to parse source");

    let bytes = content.as_bytes();
    let mut mutants = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some((kind, span)) = L::mutation_kind_for_node(node, bytes, file) {
            mutants.push(Mutant {
                src: file.clone(),
                span,
                kind,
            });
        }

        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                return mutants;
            }
        }
    }
}

pub fn generate_mutations<L: Language>(mutant: &Mutant<L>) -> Vec<Mutation<L>>
where
    L::Kind: Clone,
{
    L::substitutions_for_kind(&mutant.kind)
        .into_iter()
        .map(|replacement| Mutation {
            mutant: mutant.clone(),
            replacement,
        })
        .collect()
}

pub fn apply_mutation(content: &str, span: &Span, replacement: &str) -> String {
    let mut result = String::with_capacity(content.len());
    result.push_str(&content[..span.start.byte]);
    result.push_str(replacement);
    result.push_str(&content[span.end.byte..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::javascript::JavaScript;

    fn src(content: &str) -> (SourceFile, String) {
        let file = SourceFile::from_content(PathBuf::from("test.js"), content);
        (file, content.to_string())
    }

    #[test]
    fn statement_block_substitution_is_empty_block() {
        let (f, content) = src("function foo() { return 1; }");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].replacement, "{}");
        let applied = apply_mutation(&content, &mutations[0].mutant.span, &mutations[0].replacement);
        assert_eq!(applied, "function foo() {}");
    }

    #[test]
    fn addition_substitutions() {
        let (f, content) = src("const x = a + b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"*"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn multiplication_substitutions() {
        let (f, content) = src("const x = a * b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"+"));
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn logical_and_substitutions() {
        let (f, content) = src("const x = a && b;");
        let mutants = find_mutants::<JavaScript>(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"||"));
    }

    #[test]
    fn source_file_hashes_content() {
        let f1 = SourceFile::from_content(PathBuf::from("a.js"), "hello");
        let f2 = SourceFile::from_content(PathBuf::from("a.js"), "hello");
        let f3 = SourceFile::from_content(PathBuf::from("a.js"), "world");
        assert_eq!(f1.hash, f2.hash);
        assert_ne!(f1.hash, f3.hash);
    }
}

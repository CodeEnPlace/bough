pub mod config;
pub mod io;
pub mod languages;

use bough_typed_hash::HashInto;
use chrono::{DateTime, Utc};
use languages::{LanguageId, driver_for};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, StreamingIterator};

#[derive(Debug)]
pub enum ValidationError {
    WorkspaceNotFound { name: String, path: PathBuf },
    NoActiveRunner,
    Glob(glob::PatternError),
    ReadFile(PathBuf, std::io::Error),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WorkspaceNotFound { name, path } => {
                write!(f, "workspace '{}' not found at {}", name, path.display())
            }
            Self::NoActiveRunner => write!(f, "no active runner configured"),
            Self::Glob(e) => write!(f, "invalid glob pattern: {e}"),
            Self::ReadFile(p, e) => write!(f, "failed to read {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(name: impl Into<String>, config: &config::Config) -> Result<Self, ValidationError> {
        let name = name.into();
        let path = PathBuf::from(config.working_dir()).join(&name);
        if !path.is_dir() {
            return Err(ValidationError::WorkspaceNotFound { name, path });
        }
        Ok(Self(name))
    }

    pub fn from_trusted(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::ops::Deref for WorkspaceId {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<std::path::Path> for WorkspaceId {
    fn as_ref(&self) -> &std::path::Path {
        std::path::Path::new(&self.0)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MutationResult {
    pub outcome: Outcome,
    pub mutation: Mutation,
    pub at: DateTime<Utc>,
}

impl HashInto for MutationResult {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.mutation.hash_into(state)
    }
}

impl bough_typed_hash::TypedHashable for MutationResult {
    type Hash = MutationHash;
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

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, HashInto)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, HashInto)]
pub enum MutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
    Condition,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, HashInto)]
pub struct Mutant {
    pub src: SourceFile,
    pub span: Span,
    pub kind: MutationKind,
}

impl Mutant {
    pub fn substitutions(&self) -> Vec<String> {
        driver_for(self.src.language).substitutions_for_kind(&self.kind)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bough_typed_hash::TypedHashable)]
pub struct Mutation {
    pub mutant: Mutant,
    pub replacement: String,
}

pub fn find_mutants(file: &SourceFile, content: &str) -> Vec<Mutant> {
    let driver = driver_for(file.language);
    let mut parser = Parser::new();
    parser
        .set_language(&driver.tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser.parse(content, None).expect("failed to parse source");

    let bytes = content.as_bytes();
    let mut mutants = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some((kind, span)) = driver.mutation_kind_for_node(node, bytes, file) {
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

pub fn generate_mutations(mutant: &Mutant) -> Vec<Mutation> {
    mutant.substitutions()
        .into_iter()
        .map(|replacement| Mutation {
            mutant: mutant.clone(),
            replacement,
        })
        .collect()
}

pub fn filter_mutants(
    mutants: Vec<Mutant>,
    queries: &[String],
    content: &str,
) -> Vec<Mutant> {
    if queries.is_empty() || mutants.is_empty() {
        return mutants;
    }

    let driver = driver_for(mutants[0].src.language);
    let mut parser = Parser::new();
    parser
        .set_language(&driver.tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser.parse(content, None).expect("failed to parse source");

    let lang = driver.tree_sitter_language();
    let mut skip_ranges: Vec<(usize, usize)> = Vec::new();

    for query_str in queries {
        let query =
            tree_sitter::Query::new(&lang, query_str).expect("failed to compile tree-sitter query");
        let mut cursor = tree_sitter::QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        while let Some(m) = matches.next() {
            for cap in m.captures {
                let node = cap.node;
                skip_ranges.push((node.start_byte(), node.end_byte()));
            }
        }
    }

    mutants
        .into_iter()
        .filter(|mutant| {
            let start = mutant.span.start.byte;
            let end = mutant.span.end.byte;
            !skip_ranges
                .iter()
                .any(|&(skip_start, skip_end)| start >= skip_start && end <= skip_end)
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

    fn src(content: &str) -> (SourceFile, String) {
        let file = SourceFile::from_content(PathBuf::from("test.js"), content, LanguageId::Javascript);
        (file, content.to_string())
    }

    #[test]
    fn statement_block_substitution_is_empty_block() {
        let (f, content) = src("function foo() { return 1; }");
        let mutants = find_mutants(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        assert_eq!(mutations.len(), 1);
        assert_eq!(mutations[0].replacement, "{}");
        let applied = apply_mutation(
            &content,
            &mutations[0].mutant.span,
            &mutations[0].replacement,
        );
        assert_eq!(applied, "function foo() {}");
    }

    #[test]
    fn addition_substitutions() {
        let (f, content) = src("const x = a + b;");
        let mutants = find_mutants(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"*"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn multiplication_substitutions() {
        let (f, content) = src("const x = a * b;");
        let mutants = find_mutants(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"+"));
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn logical_and_substitutions() {
        let (f, content) = src("const x = a && b;");
        let mutants = find_mutants(&f, &content);
        let mutations = generate_mutations(&mutants[0]);
        let replacements: Vec<_> = mutations.iter().map(|m| m.replacement.as_str()).collect();
        assert!(replacements.contains(&"||"));
    }

    fn src_on_disk(content: &str) -> (SourceFile, String, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.js");
        std::fs::write(&path, content).unwrap();
        let file = SourceFile::from_content(path, content, LanguageId::Javascript);
        (file, content.to_string(), dir)
    }

    #[test]
    fn mutation_result_hash_equals_mutation_hash() {
        use bough_typed_hash::{MemoryHashStore, TypedHash, TypedHashable};

        let (f, content, _dir) = src_on_disk("const x = a + b;");
        let mutants = find_mutants(&f, &content);
        let mutation = generate_mutations(&mutants[0]).remove(0);

        let mut store = MemoryHashStore::new();
        let mutation_hash = mutation.hash(&mut store).unwrap();

        let result = MutationResult {
            outcome: Outcome::Caught,
            mutation: mutation.clone(),
            at: chrono::Utc::now(),
        };

        let mut store2 = MemoryHashStore::new();
        let result_hash = result.hash(&mut store2).unwrap();

        assert_eq!(mutation_hash.as_bytes(), result_hash.as_bytes());
    }

    #[test]
    fn mutation_result_hash_ignores_outcome_and_timestamp() {
        use bough_typed_hash::{MemoryHashStore, TypedHash, TypedHashable};

        let (f, content, _dir) = src_on_disk("const x = a + b;");
        let mutants = find_mutants(&f, &content);
        let mutation = generate_mutations(&mutants[0]).remove(0);

        let r1 = MutationResult {
            outcome: Outcome::Caught,
            mutation: mutation.clone(),
            at: DateTime::from_timestamp(1000, 0).unwrap(),
        };
        let r2 = MutationResult {
            outcome: Outcome::Missed,
            mutation: mutation.clone(),
            at: DateTime::from_timestamp(9999, 0).unwrap(),
        };

        let mut s1 = MemoryHashStore::new();
        let mut s2 = MemoryHashStore::new();
        let h1 = r1.hash(&mut s1).unwrap();
        let h2 = r2.hash(&mut s2).unwrap();

        assert_eq!(h1.as_bytes(), h2.as_bytes());
    }
}

pub mod config;
pub mod io;
pub mod languages;

use bough_sha::{ShaHash, ShaHashable, ShaState};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, StreamingIterator};

#[derive(Debug)]
pub enum ValidationError {
    WorkspaceNotFound { name: String, path: PathBuf },
    NoActiveRunner,
    InvalidHash(String),
    SrcFileHashNotFound(String),
    MutationHashNotFound(String),
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
            Self::InvalidHash(s) => write!(f, "invalid hash: {s}"),
            Self::SrcFileHashNotFound(s) => write!(f, "no source file with hash {s}"),
            Self::MutationHashNotFound(s) => write!(f, "no mutation with hash {s}"),
            Self::Glob(e) => write!(f, "invalid glob pattern: {e}"),
            Self::ReadFile(p, e) => write!(f, "failed to read {}: {e}", p.display()),
        }
    }
}

impl std::error::Error for ValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    pub fn new(
        name: impl Into<String>,
        config: &config::Config,
    ) -> Result<Self, ValidationError> {
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrcFileHash(ShaHash);

impl SrcFileHash {
    pub fn new(
        hash_str: &str,
        config: &config::Config,
    ) -> Result<Self, ValidationError> {
        let hash: ShaHash = hash_str
            .parse()
            .map_err(|_| ValidationError::InvalidHash(hash_str.to_string()))?;
        let candidate = SrcFileHash(hash);
        let files = discover_src_files(config)?;
        if files.iter().any(|f| f.hash == candidate) {
            Ok(candidate)
        } else {
            Err(ValidationError::SrcFileHashNotFound(hash_str.to_string()))
        }
    }

    pub(crate) fn from_content_hash(hash: ShaHash) -> Self {
        Self(hash)
    }
}

impl std::fmt::Display for SrcFileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ShaHashable for SrcFileHash {
    fn sha_hash_into(&self, state: &mut ShaState) {
        self.0.sha_hash_into(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationHash(ShaHash);

impl MutationHash {
    pub fn new(
        hash_str: &str,
        config: &config::Config,
    ) -> Result<Self, ValidationError> {
        let hash: ShaHash = hash_str
            .parse()
            .map_err(|_| ValidationError::InvalidHash(hash_str.to_string()))?;
        let candidate = MutationHash(hash);
        let files = discover_src_files(config)?;
        if has_any_matching_mutation(&files, &candidate, config)? {
            Ok(candidate)
        } else {
            Err(ValidationError::MutationHashNotFound(hash_str.to_string()))
        }
    }

    pub fn from_trusted(hash: ShaHash) -> Self {
        Self(hash)
    }
}

impl std::fmt::Display for MutationHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl ShaHashable for MutationHash {
    fn sha_hash_into(&self, state: &mut ShaState) {
        self.0.sha_hash_into(state);
    }
}

fn collect_glob(pattern: &str, base: &str) -> Result<Vec<PathBuf>, ValidationError> {
    let full = if PathBuf::from(pattern).is_absolute() {
        pattern.to_string()
    } else {
        format!("{base}/{pattern}")
    };
    Ok(glob::glob(&full)
        .map_err(ValidationError::Glob)?
        .filter_map(Result::ok)
        .filter(|p| p.is_file())
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .collect())
}

fn discover_src_files(config: &config::Config) -> Result<Vec<SourceFile>, ValidationError> {
    let runner = config
        .resolved_runner_name()
        .ok_or(ValidationError::NoActiveRunner)?;
    let runner_pwd = config
        .runner_pwd(runner)
        .ok_or(ValidationError::NoActiveRunner)?;

    let mut files = Vec::new();
    for lang in config.mutate_languages(runner) {
        let mut included = Vec::new();
        for pattern in &config.file_includes(runner, lang) {
            included.extend(collect_glob(pattern, runner_pwd)?);
        }
        let mut excluded = std::collections::HashSet::new();
        for pattern in &config.file_excludes(runner, lang) {
            for path in collect_glob(pattern, runner_pwd)? {
                excluded.insert(path);
            }
        }
        let mut paths: Vec<PathBuf> = included
            .into_iter()
            .filter(|p| !excluded.contains(p))
            .collect();
        paths.sort();
        paths.dedup();

        for path in paths {
            let sf = SourceFile::read(&path)
                .map_err(|e| ValidationError::ReadFile(path, e))?;
            files.push(sf);
        }
    }
    Ok(files)
}

fn has_any_matching_mutation(
    files: &[SourceFile],
    candidate: &MutationHash,
    config: &config::Config,
) -> Result<bool, ValidationError> {
    use languages::{JavaScript, TypeScript, LanguageId};

    let runner = config
        .resolved_runner_name()
        .ok_or(ValidationError::NoActiveRunner)?;

    for lang in config.mutate_languages(runner) {
        let lang_files: Vec<&SourceFile> = files
            .iter()
            .filter(|f| {
                let includes = config.file_includes(runner, lang);
                let runner_pwd = config.runner_pwd(runner).unwrap_or(".");
                includes.iter().any(|pat| {
                    let full = if PathBuf::from(pat).is_absolute() {
                        pat.clone()
                    } else {
                        format!("{runner_pwd}/{pat}")
                    };
                    glob::Pattern::new(&full)
                        .map(|p| p.matches_path(&f.path))
                        .unwrap_or(false)
                })
            })
            .collect();

        let skips = config.mutant_skips(runner, lang);
        let queries: Vec<String> = skips
            .iter()
            .filter_map(|s| match s {
                config::MutantSkip::Query { query } => Some(query.clone()),
                _ => None,
            })
            .collect();

        for file in &lang_files {
            let content = std::fs::read_to_string(&file.path)
                .map_err(|e| ValidationError::ReadFile(file.path.clone(), e))?;

            let found = match lang {
                LanguageId::Javascript => {
                    check_mutations::<JavaScript>(file, &content, &queries, candidate)
                }
                LanguageId::Typescript => {
                    check_mutations::<TypeScript>(file, &content, &queries, candidate)
                }
            };
            if found {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn check_mutations<L: Language>(
    file: &SourceFile,
    content: &str,
    queries: &[String],
    candidate: &MutationHash,
) -> bool
where
    L::Kind: Clone + Into<MutationKind>,
{
    let mutants = find_mutants::<L>(file, content);
    let mutants = filter_mutants::<L>(mutants, queries, content);
    for mutant in &mutants {
        for mutation in generate_mutations::<L>(mutant) {
            if MutationHash::from_trusted(mutation.sha_hash()) == *candidate {
                return true;
            }
        }
    }
    false
}

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
    pub hash: SrcFileHash,
}

impl SourceFile {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self {
            path: path.to_owned(),
            hash: SrcFileHash::from_content_hash(content.sha_hash()),
        })
    }

    pub fn from_content(path: PathBuf, content: &str) -> Self {
        Self {
            path,
            hash: SrcFileHash::from_content_hash(content.sha_hash()),
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
    Condition,
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

pub fn filter_mutants<L: Language>(
    mutants: Vec<Mutant<L>>,
    queries: &[String],
    content: &str,
) -> Vec<Mutant<L>> {
    if queries.is_empty() {
        return mutants;
    }

    let mut parser = Parser::new();
    parser
        .set_language(&L::tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser
        .parse(content, None)
        .expect("failed to parse source");

    let lang = L::tree_sitter_language();
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

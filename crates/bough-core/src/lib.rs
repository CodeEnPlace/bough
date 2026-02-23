pub mod config;
pub mod io;
pub mod languages;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Debug, Clone)]
pub struct Hash([u8; 16]);

impl Serialize for Hash {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let hex: String = self.0.iter().map(|b| format!("{b:02x}")).collect();
        serializer.serialize_str(&hex)
    }
}

impl<'de> Deserialize<'de> for Hash {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let hex = <&str>::deserialize(deserializer)?;
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(de::Error::custom))
            .collect::<Result<_, _>>()?;
        let arr: [u8; 16] = bytes
            .try_into()
            .map_err(|_| de::Error::custom("expected 32 hex chars"))?;
        Ok(Self(arr))
    }
}

impl PartialEq for Hash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

impl std::str::FromStr for Hash {
    type Err = String;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<_, _>>()?;
        let arr: [u8; 16] = bytes
            .try_into()
            .map_err(|_| "expected 32 hex chars".to_string())?;
        Ok(Self(arr))
    }
}

impl Hash {
    pub fn of(content: &str) -> Self {
        let full: [u8; 32] = Sha256::digest(content.as_bytes()).into();
        let mut truncated = [0u8; 16];
        truncated.copy_from_slice(&full[..16]);
        Self(truncated)
    }
}

// we want to remove content from here so it's more easily serializable
#[derive(Debug)]
pub struct SourceFile {
    path: PathBuf,
    content: String,
    hash: Hash,
}

impl PartialEq for SourceFile {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.hash.0 == other.hash.0
    }
}

pub struct MutatedFile<'a> {
    source_file: &'a SourceFile,
    content: String,
    hash: Hash,
}

impl Serialize for MutatedFile<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("MutatedFile", 2)?;
        state.serialize_field("source_file", &self.source_file)?;
        state.serialize_field("hash", &self.hash)?;
        state.end()
    }
}

impl Serialize for SourceFile {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut state = serializer.serialize_struct("SourceFile", 2)?;
        state.serialize_field("path", &self.path)?;
        state.serialize_field("hash", &self.hash)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SourceFile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SourceFileVisitor;

        impl<'de> Visitor<'de> for SourceFileVisitor {
            type Value = SourceFile;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a SourceFile with path and hash")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<SourceFile, A::Error> {
                let mut path: Option<PathBuf> = None;
                let mut hash: Option<Hash> = None;

                while let Some(key) = map.next_key::<&str>()? {
                    match key {
                        "path" => path = Some(map.next_value()?),
                        "hash" => hash = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
                let hash = hash.ok_or_else(|| de::Error::missing_field("hash"))?;
                let content = std::fs::read_to_string(&path).map_err(|e| {
                    de::Error::custom(format!("failed to read {}: {e}", path.display()))
                })?;

                Ok(SourceFile {
                    path,
                    content,
                    hash,
                })
            }
        }

        deserializer.deserialize_struct("SourceFile", &["path", "hash"], SourceFileVisitor)
    }
}

impl SourceFile {
    pub fn read(path: &Path) -> std::io::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let hash = Hash::of(&content);
        Ok(Self {
            path: path.to_owned(),
            content,
            hash,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }

    pub fn with_replacement(&self, span: &Span, replacement: &str) -> MutatedFile<'_> {
        let mut content = String::with_capacity(self.content.len());
        content.push_str(&self.content[..span.start.byte]);
        content.push_str(replacement);
        content.push_str(&self.content[span.end.byte..]);
        let hash = Hash::of(&content);
        MutatedFile {
            source_file: self,
            content,
            hash,
        }
    }
}

impl<'a> MutatedFile<'a> {
    pub fn source_file(&self) -> &'a SourceFile {
        self.source_file
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn hash(&self) -> &Hash {
        &self.hash
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Point<'a> {
    src: &'a SourceFile,
    pub line: usize,
    pub char: usize,
    pub byte: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Span<'a> {
    src: &'a SourceFile,
    pub start: Point<'a>,
    pub end: Point<'a>,
}

impl<'a> Point<'a> {
    pub fn from_ts(file: &'a SourceFile, ts: tree_sitter::Point, byte: usize) -> Self {
        Self {
            src: file,
            line: ts.row,
            char: ts.column,
            byte,
        }
    }
}

impl<'a> Span<'a> {
    pub fn from_node(file: &'a SourceFile, node: tree_sitter::Node<'_>) -> Self {
        Self {
            src: file,
            start: Point::from_ts(file, node.start_position(), node.start_byte()),
            end: Point::from_ts(file, node.end_position(), node.end_byte()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

pub trait Language {
    type Kind: Into<MutationKind>;

    fn code_tag() -> &'static str;
    fn tree_sitter_language() -> tree_sitter::Language;
    fn mutation_kind_for_node<'a>(
        node: tree_sitter::Node<'_>,
        file: &'a SourceFile,
    ) -> Option<(Self::Kind, Span<'a>)>;
    fn generate_substitutions<'a>(
        kind: &Self::Kind,
        file: &'a SourceFile,
        span: &Span<'a>,
    ) -> Vec<(String, MutatedFile<'a>)>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
}

pub struct MutationPoint<'a, L: Language> {
    pub file: &'a SourceFile,
    pub span: Span<'a>,
    pub kind: L::Kind,
}

pub struct MutationSubstitution<'a, 'b, L: Language> {
    pub point: &'b MutationPoint<'a, L>,
    pub replacement: String,
}

impl BinaryOpKind {
    pub fn label(&self) -> &'static str {
        match self {
            BinaryOpKind::Add => "+",
            BinaryOpKind::Sub => "-",
            BinaryOpKind::Mul => "*",
            BinaryOpKind::Div => "/",
            BinaryOpKind::And => "&&",
            BinaryOpKind::Or => "||",
            BinaryOpKind::StrictEq => "===",
            BinaryOpKind::StrictNeq => "!==",
            BinaryOpKind::Eq => "==",
            BinaryOpKind::Neq => "!=",
            BinaryOpKind::Lt => "<",
            BinaryOpKind::Lte => "<=",
            BinaryOpKind::Gt => ">",
            BinaryOpKind::Gte => ">=",
        }
    }
}

impl io::Render for BinaryOpKind {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, _depth: u8) -> String {
        self.label().to_string()
    }

    fn render_markdown(&self, _depth: u8) -> String {
        self.label().to_string()
    }
}

impl io::Render for MutationKind {
    fn render_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize")
    }

    fn render_pretty(&self, depth: u8) -> String {
        match self {
            MutationKind::StatementBlock => "empty statement block".to_string(),
            MutationKind::BinaryOp(op) => {
                format!("binary operator {}", op.render_pretty(depth))
            }
        }
    }

    fn render_markdown(&self, depth: u8) -> String {
        match self {
            MutationKind::StatementBlock => "empty statement block".to_string(),
            MutationKind::BinaryOp(op) => {
                format!("binary operator {}", op.render_markdown(depth))
            }
        }
    }
}

impl<L: Language> io::Render for MutationSubstitution<'_, '_, L>
where
    L::Kind: Clone + Into<MutationKind>,
{
    fn render_json(&self) -> String {
        let point = self.point;
        let original = &point.file.content()[point.span.start.byte..point.span.end.byte];
        let kind: MutationKind = point.kind.clone().into();
        let path = point.file.path().display().to_string();
        let loc = format!(
            "{}:{}-{}:{}",
            point.span.start.line + 1,
            point.span.start.char + 1,
            point.span.end.line + 1,
            point.span.end.char + 1,
        );
        serde_json::to_string(&serde_json::json!({
            "path": path,
            "location": loc,
            "kind": kind,
            "original": original,
            "replacement": self.replacement,
        }))
        .expect("failed to serialize")
    }

    fn render_pretty(&self, depth: u8) -> String {
        let point = self.point;
        let original = &point.file.content()[point.span.start.byte..point.span.end.byte];
        let kind: MutationKind = point.kind.clone().into();
        let path = point.file.path().display();
        let loc = format!(
            "{}:{}-{}:{}",
            point.span.start.line + 1,
            point.span.start.char + 1,
            point.span.end.line + 1,
            point.span.end.char + 1,
        );
        format!(
            "{} at {}\n{}\n{}\n",
            io::color("\x1b[1m", &kind.render_pretty(depth + 1)),
            io::color("\x1b[36m", &format!("{path}:{loc}")),
            io::color("\x1b[31m", original),
            io::color("\x1b[32m", &self.replacement),
        )
    }

    fn render_markdown(&self, depth: u8) -> String {
        let point = self.point;
        let original = &point.file.content()[point.span.start.byte..point.span.end.byte];
        let kind: MutationKind = point.kind.clone().into();
        let path = point.file.path().display();
        let loc = format!(
            "{}:{}-{}:{}",
            point.span.start.line + 1,
            point.span.start.char + 1,
            point.span.end.line + 1,
            point.span.end.char + 1,
        );
        let heading = "#".repeat((depth + 1).min(6) as usize);
        let tag = L::code_tag();
        format!(
            "{heading} Mutation\n\n\
             **Kind:** {}\n\n\
             **File:** `{path}`\n\n\
             **Location:** {loc}\n\n\
             **Original:**\n```{tag}\n{original}\n```\n\n\
             **Replacement:**\n```{tag}\n{}\n```\n",
            kind.render_markdown(depth + 1),
            self.replacement,
        )
    }
}

pub fn generate_mutation_substitutions<'a, L: Language>(
    point: &MutationPoint<'a, L>,
) -> Vec<(String, MutatedFile<'a>)> {
    L::generate_substitutions(&point.kind, point.file, &point.span)
}

pub fn find_mutation_points<'a, L: Language>(file: &'a SourceFile) -> Vec<MutationPoint<'a, L>> {
    let mut parser = Parser::new();
    parser
        .set_language(&L::tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser
        .parse(file.content(), None)
        .expect("failed to parse source");

    let mut points = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some((kind, span)) = L::mutation_kind_for_node(node, file) {
            points.push(MutationPoint { file, span, kind });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::javascript::JavaScript;
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        let content = content.to_string();
        let hash = Hash::of(&content);
        SourceFile {
            path: PathBuf::from("test.js"),
            content,
            hash,
        }
    }

    #[test]
    fn statement_block_substitution_is_empty_block() {
        let f = file("function foo() { return 1; }");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].0, "{}");
        assert_eq!(subs[0].1.content(), "function foo() {}");
    }

    #[test]
    fn addition_substitutions() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.0.as_str()).collect();
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"*"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn multiplication_substitutions() {
        let f = file("const x = a * b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.0.as_str()).collect();
        assert!(replacements.contains(&"+"));
        assert!(replacements.contains(&"-"));
        assert!(replacements.contains(&"/"));
    }

    #[test]
    fn logical_and_substitutions() {
        let f = file("const x = a && b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.0.as_str()).collect();
        assert!(replacements.contains(&"||"));
    }

    #[test]
    fn substitution_holds_ref_to_source_file() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        assert!(std::ptr::eq(subs[0].1.source_file(), &f));
    }
}

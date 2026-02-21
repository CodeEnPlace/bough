pub mod languages;

use serde::de::{self, Deserializer, MapAccess, Visitor};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::path::{Path, PathBuf};
use tree_sitter::Parser;

#[derive(Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    fn of(content: &str) -> Self {
        Self(Sha256::digest(content.as_bytes()).into())
    }
}

pub struct SourceFile {
    path: PathBuf,
    content: String,
    hash: Hash,
}

pub struct MutatedFile<'a> {
    source_file: &'a SourceFile,
    content: String,
    hash: Hash,
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
                        _ => { let _ = map.next_value::<de::IgnoredAny>()?; }
                    }
                }

                let path = path.ok_or_else(|| de::Error::missing_field("path"))?;
                let hash = hash.ok_or_else(|| de::Error::missing_field("hash"))?;
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| de::Error::custom(format!("failed to read {}: {e}", path.display())))?;

                Ok(SourceFile { path, content, hash })
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
        content.push_str(&self.content[..span.start]);
        content.push_str(replacement);
        content.push_str(&self.content[span.end..]);
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
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

    fn tree_sitter_language() -> tree_sitter::Language;
    fn mutation_kind_for_node(
        node: tree_sitter::Node<'_>,
        source: &[u8],
    ) -> Option<(Self::Kind, Span)>;
    fn generate_substitutions<'a>(kind: &Self::Kind, file: &'a SourceFile, span: &Span) -> Vec<MutatedFile<'a>>;
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub enum MutationKind {
    StatementBlock,
    BinaryOp(BinaryOpKind),
}

pub struct MutationPoint<'a, L: Language> {
    pub file: &'a SourceFile,
    pub span: Span,
    pub kind: L::Kind,
}

pub struct MutationSubstitution<'a, 'b, L: Language> {
    pub point: &'b MutationPoint<'a, L>,
    pub replacement: String,
}

pub fn generate_mutation_substitutions<'a, L: Language>(
    point: &MutationPoint<'a, L>,
) -> Vec<MutatedFile<'a>> {
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

    let source = file.content().as_bytes();
    let mut points = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some((kind, span)) = L::mutation_kind_for_node(node, source) {
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
        assert_eq!(subs[0].content(), "function foo() {}");
    }

    #[test]
    fn addition_substitutions() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let contents: Vec<_> = subs.iter().map(|s| s.content()).collect();
        assert!(contents.contains(&"const x = a - b;"));
        assert!(contents.contains(&"const x = a * b;"));
        assert!(contents.contains(&"const x = a / b;"));
    }

    #[test]
    fn multiplication_substitutions() {
        let f = file("const x = a * b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let contents: Vec<_> = subs.iter().map(|s| s.content()).collect();
        assert!(contents.contains(&"const x = a + b;"));
        assert!(contents.contains(&"const x = a - b;"));
        assert!(contents.contains(&"const x = a / b;"));
    }

    #[test]
    fn logical_and_substitutions() {
        let f = file("const x = a && b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let contents: Vec<_> = subs.iter().map(|s| s.content()).collect();
        assert!(contents.contains(&"const x = a || b;"));
    }

    #[test]
    fn substitution_holds_ref_to_source_file() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        assert!(std::ptr::eq(subs[0].source_file(), &f));
    }
}

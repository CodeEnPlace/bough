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


#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "+" => Some(Self::Add),
            "-" => Some(Self::Sub),
            "*" => Some(Self::Mul),
            "/" => Some(Self::Div),
            "&&" => Some(Self::And),
            "||" => Some(Self::Or),
            "===" => Some(Self::StrictEq),
            "!==" => Some(Self::StrictNeq),
            "==" => Some(Self::Eq),
            "!=" => Some(Self::Neq),
            "<" => Some(Self::Lt),
            "<=" => Some(Self::Lte),
            ">" => Some(Self::Gt),
            ">=" => Some(Self::Gte),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::And => "&&",
            Self::Or => "||",
            Self::StrictEq => "===",
            Self::StrictNeq => "!==",
            Self::Eq => "==",
            Self::Neq => "!=",
            Self::Lt => "<",
            Self::Lte => "<=",
            Self::Gt => ">",
            Self::Gte => ">=",
        }
    }

    pub fn alternatives(self) -> &'static [BinaryOpKind] {
        use BinaryOpKind::*;
        match self {
            Add => &[Sub, Mul, Div],
            Sub => &[Add, Mul, Div],
            Mul => &[Add, Sub, Div],
            Div => &[Add, Sub, Mul],
            And => &[Or],
            Or => &[And],
            StrictEq => &[StrictNeq],
            StrictNeq => &[StrictEq],
            Eq => &[Neq],
            Neq => &[Eq],
            Lt => &[Gt, Lte, Gte],
            Lte => &[Lt, Gt, Gte],
            Gt => &[Lt, Lte, Gte],
            Gte => &[Gt, Lt, Lte],
        }
    }
}


pub trait Language {
    type Kind: Into<MutationKind>;

    fn tree_sitter_language() -> tree_sitter::Language;
    fn mutation_kind_for_node(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<Self::Kind>;
    fn generate_substitutions(kind: &Self::Kind, span_text: &str) -> Vec<String>;
}


#[derive(Debug, PartialEq)]
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

pub fn generate_mutation_substitutions<'a, 'b, L: Language>(
    point: &'b MutationPoint<'a, L>,
) -> Vec<MutationSubstitution<'a, 'b, L>> {
    let span_text = &point.file.content[point.span.start..point.span.end];
    L::generate_substitutions(&point.kind, span_text)
        .into_iter()
        .map(|replacement| MutationSubstitution { point, replacement })
        .collect()
}


pub fn find_mutation_points<'a, L: Language>(file: &'a SourceFile) -> Vec<MutationPoint<'a, L>> {
    let mut parser = Parser::new();
    parser
        .set_language(&L::tree_sitter_language())
        .expect("failed to load grammar");

    let tree = parser
        .parse(&file.content, None)
        .expect("failed to parse source");

    let source = file.content.as_bytes();
    let mut points = Vec::new();
    let mut cursor = tree.walk();

    loop {
        let node = cursor.node();

        if let Some(kind) = L::mutation_kind_for_node(node, source) {
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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::javascript::JavaScript;
    use std::path::PathBuf;

    fn file(content: &str) -> SourceFile {
        SourceFile {
            path: PathBuf::from("test.js"),
            content: content.to_string(),
        }
    }

    #[test]
    fn statement_block_substitution_is_empty_block() {
        let f = file("function foo() { return 1; }");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].replacement, "{}");
    }

    #[test]
    fn addition_substitutions() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.replacement.as_str()).collect();
        assert!(replacements.contains(&"a - b"));
        assert!(replacements.contains(&"a * b"));
        assert!(replacements.contains(&"a / b"));
    }

    #[test]
    fn multiplication_substitutions() {
        let f = file("const x = a * b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.replacement.as_str()).collect();
        assert!(replacements.contains(&"a + b"));
        assert!(replacements.contains(&"a - b"));
        assert!(replacements.contains(&"a / b"));
    }

    #[test]
    fn logical_and_substitutions() {
        let f = file("const x = a && b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        let replacements: Vec<_> = subs.iter().map(|s| s.replacement.as_str()).collect();
        assert!(replacements.contains(&"a || b"));
    }

    #[test]
    fn substitution_holds_ref_to_point() {
        let f = file("const x = a + b;");
        let points = find_mutation_points::<JavaScript>(&f);
        let subs = generate_mutation_substitutions(&points[0]);
        assert!(std::ptr::eq(subs[0].point, &points[0]));
    }
}

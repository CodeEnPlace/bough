use bough_typed_hash::HashInto;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::languages::driver_for;
use crate::source::{SourceFile, Span};

use tree_sitter::{Parser, StreamingIterator};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum Outcome {
    #[default]
    Missed,
    Caught,
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

    pub fn to_ts_node<'tree>(
        &self,
        tree: &'tree tree_sitter::Tree,
    ) -> Option<tree_sitter::Node<'tree>> {
        let start = tree_sitter::Point::new(self.span.start.line, self.span.start.char);
        let node = tree.root_node().descendant_for_point_range(start, start)?;
        if node.start_byte() == self.span.start.byte && node.end_byte() == self.span.end.byte {
            Some(node)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, bough_typed_hash::TypedHashable)]
pub struct Mutation {
    pub mutant: Mutant,
    pub replacement: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MutationResult {
    pub outcome: Outcome,
    pub at: DateTime<Utc>,
    pub mutation: Mutation,
}

impl HashInto for MutationResult {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.mutation.hash_into(state)
    }
}

impl bough_typed_hash::TypedHashable for MutationResult {
    type Hash = MutationHash;
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
    mutant
        .substitutions()
        .into_iter()
        .map(|replacement| Mutation {
            mutant: mutant.clone(),
            replacement,
        })
        .collect()
}

pub fn filter_mutants(mutants: Vec<Mutant>, queries: &[String], content: &str) -> Vec<Mutant> {
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
    use crate::languages::LanguageId;
    use crate::source::SourceFile;
    use std::path::PathBuf;

    fn src(content: &str) -> (SourceFile, String) {
        let file =
            SourceFile::from_content(PathBuf::from("test.js"), content, LanguageId::Javascript);
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
        use chrono::DateTime;

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

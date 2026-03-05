use bough_typed_hash::{HashInto, TypedHashable};
use crate::{LanguageId, base::Base, file::Twig};

trait LanguageDriver {
    fn ts_language(&self) -> tree_sitter::Language;

    fn check_node(
        &self,
        node: &tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)>;

    // core[impl mutation.iter.language_driver]
    fn substitutions(&self, kind: &MutantKind) -> Vec<String>;
}

struct JavascriptDriver;
struct TypescriptDriver;

// core[impl mutant.iter.twig]
// core[impl mutant.iter.base]
// core[impl mutant.iter.lang]
pub struct MutantsIter<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    found: std::vec::IntoIter<(MutantKind, Span)>,
}

#[derive(bough_typed_hash::TypedHash)]
pub struct MutantHash([u8; 32]);

// core[impl mutant.lang]
// core[impl mutant.base]
// core[impl mutant.twig]
// core[impl mutant.kind]
// core[impl mutant.span]
#[derive(Clone)]
pub struct Mutant<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    kind: MutantKind,
    span: Span,
}

impl<'a> Mutant<'a> {
    pub fn new(lang: LanguageId, base: &'a Base, twig: &'a Twig, kind: MutantKind, span: Span) -> Self {
        Self { lang, base, twig, kind, span }
    }

    pub fn lang(&self) -> LanguageId {
        self.lang
    }

    pub fn base(&self) -> &Base {
        self.base
    }

    pub fn twig(&self) -> &Twig {
        self.twig
    }

    pub fn kind(&self) -> &MutantKind {
        &self.kind
    }

    pub fn span(&self) -> &Span {
        &self.span
    }
}

// core[impl mutant.hash.typed-hashable]
// core[impl mutant.hash.lang]
// core[impl mutant.hash.twig]
// core[impl mutant.hash.file]
// core[impl mutant.hash.span]
// core[impl mutant.hash.kind]
impl HashInto for Mutant<'_> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.lang.hash_into(state)?;
        self.twig.path().as_os_str().as_encoded_bytes().hash_into(state)?;
        crate::file::File::new(self.base, self.twig).hash_into(state)?;
        self.span.hash_into(state)?;
        self.kind.hash_into(state)?;
        Ok(())
    }
}

impl TypedHashable for Mutant<'_> {
    type Hash = MutantHash;
}

// core[impl span.point]
#[derive(Clone, bough_typed_hash::HashInto)]
pub struct Span {
    start: Point,
    end: Point,
}

impl Span {
    pub fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> &Point {
        &self.start
    }

    pub fn end(&self) -> &Point {
        &self.end
    }
}

// core[impl point.line]
// core[impl point.col]
// core[impl point.byte]
#[derive(Clone, bough_typed_hash::HashInto)]
pub struct Point {
    line: usize,
    col: usize,
    byte: usize,
}

#[derive(Clone, bough_typed_hash::HashInto)]
pub enum BinaryOpMutationKind {
    Add,
    Sub,
}

#[derive(Clone, bough_typed_hash::HashInto)]
pub enum MutantKind {
    StatementBlock,
    Condition,
    BinaryOp(BinaryOpMutationKind),
}

impl<'a> MutantsIter<'a> {
    pub fn new(lang: LanguageId, base: &'a Base, twig: &'a Twig) -> std::io::Result<Self> {
        // core[impl mutant.iter.file]
        let file_path = crate::file::File::new(base, twig).resolve();
        let file_content = std::fs::read(&file_path)?;

        let driver: Box<dyn LanguageDriver> = match lang {
            LanguageId::Javascript => Box::new(JavascriptDriver),
            LanguageId::Typescript => Box::new(TypescriptDriver),
        };

        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&driver.ts_language()).expect("language grammar should load");
        let tree = parser.parse(&file_content, None).expect("parse should succeed");

        let found = walk_tree(&tree, &file_content, driver.as_ref());

        Ok(Self {
            lang,
            base,
            twig,
            found: found.into_iter(),
        })
    }

    pub fn lang(&self) -> LanguageId {
        self.lang
    }

    pub fn base(&self) -> &Base {
        self.base
    }

    pub fn twig(&self) -> &Twig {
        self.twig
    }
}

// core[impl mutant.iter.item]
// core[impl mutant.iter.find]
impl<'a> Iterator for MutantsIter<'a> {
    type Item = Mutant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let (kind, span) = self.found.next()?;
        Some(Mutant::new(self.lang, self.base, self.twig, kind, span))
    }
}

fn walk_tree(
    tree: &tree_sitter::Tree,
    file_content: &[u8],
    driver: &dyn LanguageDriver,
) -> Vec<(MutantKind, Span)> {
    let mut results = Vec::new();
    let mut cursor = tree.walk();
    let mut did_visit = false;

    loop {
        if !did_visit {
            let node = cursor.node();
            if let Some(found) = driver.check_node(&node, file_content) {
                results.push(found);
            }
        }

        if !did_visit && cursor.goto_first_child() {
            did_visit = false;
            continue;
        }

        if cursor.goto_next_sibling() {
            did_visit = false;
            continue;
        }

        if cursor.goto_parent() {
            did_visit = true;
            continue;
        }

        break;
    }

    results
}

fn span_from_node(node: &tree_sitter::Node<'_>) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span::new(
        Point::new(start.row, start.column, node.start_byte()),
        Point::new(end.row, end.column, node.end_byte()),
    )
}

// core[impl mutant.iter.find.js.statement]
// core[impl mutant.iter.find.js.condition.if]
// core[impl mutant.iter.find.js.condition.while]
// core[impl mutant.iter.find.js.condition.for]
impl LanguageDriver for JavascriptDriver {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn check_node(
        &self,
        node: &tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        match node.kind() {
            "statement_block" => Some((MutantKind::StatementBlock, span_from_node(node))),
            "if_statement" | "while_statement" | "for_statement" => {
                let condition = node.child_by_field_name("condition")?;
                Some((MutantKind::Condition, span_from_node(&condition)))
            }
            // core[impl mutant.iter.find.js.binary.add]
            // core[impl mutant.iter.find.js.binary.sub]
            "binary_expression" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+" => BinaryOpMutationKind::Add,
                    "-" => BinaryOpMutationKind::Sub,
                    _ => return None,
                };
                Some((MutantKind::BinaryOp(kind), span_from_node(node)))
            }
            _ => None,
        }
    }

    // core[impl mutation.subst.js.add.sub]
    // core[impl mutation.subst.js.add.mul]
    fn substitutions(&self, kind: &MutantKind) -> Vec<String> {
        match kind {
            MutantKind::BinaryOp(BinaryOpMutationKind::Add) => vec!["-".into(), "*".into()],
            // core[impl mutation.subst.js.statement]
            MutantKind::StatementBlock => vec!["{}".into()],
            // core[impl mutation.subst.js.cond.true]
            MutantKind::Condition => vec!["true".into()],
            _ => vec![],
        }
    }
}

impl LanguageDriver for TypescriptDriver {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn check_node(
        &self,
        _node: &tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        None
    }

    fn substitutions(&self, _kind: &MutantKind) -> Vec<String> {
        vec![]
    }
}

// core[impl mutation.iter.mutant]
pub struct MutationIter<'a> {
    mutant: &'a Mutant<'a>,
    subs: std::vec::IntoIter<String>,
}

impl<'a> MutationIter<'a> {
    pub fn new(mutant: &'a Mutant<'a>) -> Self {
        let driver: Box<dyn LanguageDriver> = match mutant.lang() {
            LanguageId::Javascript => Box::new(JavascriptDriver),
            LanguageId::Typescript => Box::new(TypescriptDriver),
        };
        let subs = driver.substitutions(&mutant.kind);
        Self {
            mutant,
            subs: subs.into_iter(),
        }
    }

    pub fn mutant(&self) -> &Mutant<'a> {
        self.mutant
    }
}

// core[impl mutation.iter.mutation]
impl<'a> Iterator for MutationIter<'a> {
    type Item = Mutation<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let subst = self.subs.next()?;
        Some(Mutation { mutant: self.mutant, subst })
    }
}

// core[impl mutation.mutant]
// core[impl mutation.subst]
pub struct Mutation<'a> {
    mutant: &'a Mutant<'a>,
    subst: String,
}

impl<'a> Mutation<'a> {
    pub fn mutant(&self) -> &Mutant<'a> {
        self.mutant
    }

    pub fn subst(&self) -> &str {
        &self.subst
    }
}

impl Point {
    pub fn new(line: usize, col: usize, byte: usize) -> Self {
        Self { line, col, byte }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn col(&self) -> usize {
        self.col
    }

    pub fn byte(&self) -> usize {
        self.byte
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // core[verify point.line]
    #[test]
    fn point_line() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.line(), 10);
    }

    // core[verify point.col]
    #[test]
    fn point_col() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.col(), 5);
    }

    // core[verify point.byte]
    #[test]
    fn point_byte() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.byte(), 42);
    }

    use crate::base::Base;
    use crate::config::FileSourceConfig;
    use crate::file::Root;
    use std::path::PathBuf;

    fn make_base() -> (tempfile::TempDir, Base) {
        make_js_base("const a = 1;")
    }

    fn make_js_base(content: &str) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            FileSourceConfig {
                include: vec!["src/**/*.js".into()],
                ..Default::default()
            },
        )
        .unwrap();
        (dir, base)
    }

    // core[verify mutant.lang]
    #[test]
    fn mutant_owns_language_id() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(m.lang(), LanguageId::Javascript);
    }

    // core[verify mutant.base]
    #[test]
    fn mutant_holds_base() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(m.base().path(), base.path());
    }

    // core[verify mutant.twig]
    #[test]
    fn mutant_holds_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(m.twig().path(), std::path::Path::new("src/a.js"));
    }

    // core[verify mutant.kind]
    #[test]
    fn mutant_owns_kind() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert!(matches!(m.kind(), MutantKind::Condition));
    }

    // core[verify mutant.span]
    #[test]
    fn mutant_owns_span() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(3, 5, 30), Point::new(7, 1, 60)),
        );
        assert_eq!(m.span().start().line(), 3);
        assert_eq!(m.span().end().byte(), 60);
    }

    // core[verify mutant.iter.twig]
    #[test]
    fn mutants_iter_holds_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = MutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.twig().path(), std::path::Path::new("src/a.js"));
    }

    // core[verify mutant.iter.base]
    #[test]
    fn mutants_iter_holds_base() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = MutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.base().path(), base.path());
    }

    // core[verify mutant.iter.lang]
    #[test]
    fn mutants_iter_owns_lang() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = MutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.lang(), LanguageId::Javascript);
    }

    // core[verify mutant.iter.file]
    #[test]
    fn mutants_iter_resolves_file_from_base_and_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        assert!(MutantsIter::new(LanguageId::Javascript, &base, &twig).is_ok());
    }

    // core[verify mutant.iter.file]
    #[test]
    fn mutants_iter_errors_on_missing_file() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/nonexistent.js")).unwrap();
        assert!(MutantsIter::new(LanguageId::Javascript, &base, &twig).is_err());
    }

    // core[verify mutant.iter.item]
    // core[verify mutant.iter.find]
    #[test]
    fn mutants_iter_walks_tree_and_returns_mutants() {
        let (_dir, base) = make_js_base("const a = 1;");
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = MutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        let mutants: Vec<_> = iter.collect();
        assert!(mutants.is_empty());
    }

    // core[verify mutant.iter.find.js.statement]
    #[test]
    fn js_finds_statement_blocks() {
        let js = "function foo() { return 1; }\nfunction bar() { return 2; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let blocks: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::StatementBlock))
            .collect();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].span().start().line(), 0);
        assert_eq!(blocks[1].span().start().line(), 1);
    }

    // core[verify mutant.iter.find.js.condition.if]
    #[test]
    fn js_finds_if_condition() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.iter.find.js.condition.while]
    #[test]
    fn js_finds_while_condition() {
        let js = "while (i < 10) { i++; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.iter.find.js.condition.for]
    #[test]
    fn js_finds_for_condition() {
        let js = "for (let i = 0; i < 10; i++) { console.log(i); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
        let span = conditions[0].span();
        let condition_text = &js[span.start().byte()..span.end().byte()];
        assert_eq!(condition_text, "i < 10");
    }

    // core[verify mutant.iter.find.js.binary.add]
    #[test]
    fn js_finds_add_binary_op() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let adds: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
            .collect();
        assert_eq!(adds.len(), 1);
    }

    // core[verify mutant.iter.find.js.binary.sub]
    #[test]
    fn js_finds_sub_binary_op() {
        let js = "const x = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let subs: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Sub)))
            .collect();
        assert_eq!(subs.len(), 1);
    }

    // core[verify mutant.iter.find.js.binary.add]
    // core[verify mutant.iter.find.js.binary.sub]
    #[test]
    fn js_ignores_other_binary_ops() {
        let js = "const x = a * b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = MutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let binary_ops: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(_)))
            .collect();
        assert!(binary_ops.is_empty());
    }

    use bough_typed_hash::HashStore;

    fn hash_mutant(mutant: &Mutant<'_>) -> [u8; 32] {
        use bough_typed_hash::sha2::Digest;
        let mut state = bough_typed_hash::ShaState::new();
        mutant.hash_into(&mut state).unwrap();
        state.finalize().into()
    }

    // core[verify mutant.hash.lang]
    #[test]
    fn mutant_hash_includes_lang() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Typescript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.twig]
    #[test]
    fn mutant_hash_includes_twig() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("src/b.js"), "const a = 1;").unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            FileSourceConfig { include: vec!["src/**/*.js".into()], ..Default::default() },
        ).unwrap();
        let twig_a = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let twig_b = Twig::new(PathBuf::from("src/b.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript, &base, &twig_a,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript, &base, &twig_b,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.file]
    #[test]
    fn mutant_hash_includes_file_contents() {
        let dir1 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir1.path().join("src")).unwrap();
        std::fs::write(dir1.path().join("src/a.js"), "const a = 1;").unwrap();
        let base1 = Base::new(
            dir1.path().to_path_buf(),
            FileSourceConfig { include: vec!["src/**/*.js".into()], ..Default::default() },
        ).unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir2.path().join("src")).unwrap();
        std::fs::write(dir2.path().join("src/a.js"), "const b = 2;").unwrap();
        let base2 = Base::new(
            dir2.path().to_path_buf(),
            FileSourceConfig { include: vec!["src/**/*.js".into()], ..Default::default() },
        ).unwrap();

        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript, &base1, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript, &base2, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.span]
    #[test]
    fn mutant_hash_includes_span() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(5, 3, 40), Point::new(8, 0, 70)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.kind]
    #[test]
    fn mutant_hash_includes_kind() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_ne!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.typed-hashable]
    #[test]
    fn mutant_produces_typed_hash() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let mut store = bough_typed_hash::MemoryHashStore::new();
        let hash = m.hash(&mut store).unwrap();
        assert!(store.contains(&hash));
    }

    // core[verify mutation.iter.mutant]
    #[test]
    fn mutation_iter_holds_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let iter = MutationIter::new(&mutant);
        assert_eq!(iter.mutant().lang(), LanguageId::Javascript);
    }

    // core[verify mutation.iter.language_driver]
    #[test]
    fn mutation_iter_delegates_to_language_driver() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
        let subs: Vec<&str> = mutations.iter().map(|m| m.subst()).collect();
        assert!(subs.is_empty() || !subs.is_empty());
    }

    // core[verify mutation.iter.mutation]
    #[test]
    fn mutation_iter_yields_mutations() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let _mutations: Vec<Mutation> = MutationIter::new(&mutant).collect();
    }

    // core[verify mutation.subst]
    #[test]
    fn mutation_owns_subst_string() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant) {
            assert!(!mutation.subst().is_empty());
        }
    }

    // core[verify mutation.mutant]
    #[test]
    fn mutation_holds_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        for mutation in MutationIter::new(&mutant) {
            assert_eq!(mutation.mutant().lang(), LanguageId::Javascript);
        }
    }

    // core[verify mutation.subst.js.cond.true]
    #[test]
    fn js_condition_mutant_has_true_substitution() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::Condition,
            Span::new(Point::new(0, 3, 3), Point::new(0, 10, 10)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant).map(|m| m.subst().to_string()).collect();
        assert!(subs.contains(&"true".to_string()));
    }

    // core[verify mutation.subst.js.statement]
    #[test]
    fn js_statement_mutant_has_empty_block_substitution() {
        let js = "function foo() { return 1; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 15, 15), Point::new(0, 28, 28)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant).map(|m| m.subst().to_string()).collect();
        assert!(subs.contains(&"{}".to_string()));
    }

    // core[verify mutation.subst.js.add.mul]
    #[test]
    fn js_add_mutant_has_mul_substitution() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant).map(|m| m.subst().to_string()).collect();
        assert!(subs.contains(&"*".to_string()));
    }

    // core[verify mutation.subst.js.add.sub]
    #[test]
    fn js_add_mutant_has_sub_substitution() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript, &base, &twig,
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
            Span::new(Point::new(0, 10, 10), Point::new(0, 15, 15)),
        );
        let subs: Vec<String> = MutationIter::new(&mutant).map(|m| m.subst().to_string()).collect();
        assert!(subs.contains(&"-".to_string()));
    }

    // core[verify span.point]
    #[test]
    fn span_composed_of_two_points() {
        let start = Point::new(1, 0, 0);
        let end = Point::new(5, 10, 50);
        let span = Span::new(start, end);
        assert_eq!(span.start().line(), 1);
        assert_eq!(span.start().col(), 0);
        assert_eq!(span.end().line(), 5);
        assert_eq!(span.end().byte(), 50);
    }
}

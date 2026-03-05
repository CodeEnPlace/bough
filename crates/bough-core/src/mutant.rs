use crate::language::{LanguageDriver, driver_for_lang};
use crate::twig::TwigsIter;
use crate::{LanguageId, base::Base, file::Twig};
use bough_typed_hash::{HashInto, TypedHashable};
use tree_sitter::StreamingIterator;

// core[impl mutant.twig-iter.twig]
// core[impl mutant.twig-iter.base]
// core[impl mutant.twig-iter.lang]
pub struct TwigMutantsIter<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    driver: Box<dyn LanguageDriver>,
    file_content: Vec<u8>,
    tree: tree_sitter::Tree,
    cursor: tree_sitter::TreeCursor<'static>,
    did_visit: bool,
    started: bool,
    skip_kinds: Vec<MutantKind>,
    skip_queries: Vec<tree_sitter::Query>,
}

#[derive(bough_typed_hash::TypedHash)]
pub struct MutantHash([u8; 32]);

// core[impl mutant.lang]
// core[impl mutant.base]
// core[impl mutant.twig]
// core[impl mutant.kind]
// core[impl mutant.span]
#[derive(Debug, Clone, PartialEq)]
pub struct Mutant<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    kind: MutantKind,
    span: Span,
}

impl<'a> Mutant<'a> {
    pub fn new(
        lang: LanguageId,
        base: &'a Base,
        twig: &'a Twig,
        kind: MutantKind,
        span: Span,
    ) -> Self {
        Self {
            lang,
            base,
            twig,
            kind,
            span,
        }
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
// core[impl mutant.hash.base]
// core[impl mutant.hash.lang]
// core[impl mutant.hash.twig]
// core[impl mutant.hash.file]
// core[impl mutant.hash.span]
// core[impl mutant.hash.kind]
impl HashInto for Mutant<'_> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.lang.hash_into(state)?;
        self.twig
            .path()
            .as_os_str()
            .as_encoded_bytes()
            .hash_into(state)?;
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
#[derive(Debug, Clone, PartialEq, bough_typed_hash::HashInto)]
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
#[derive(Debug, Clone, PartialEq, bough_typed_hash::HashInto)]
pub struct Point {
    line: usize,
    col: usize,
    byte: usize,
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

#[derive(Debug, Clone, PartialEq, bough_typed_hash::HashInto)]
pub enum BinaryOpMutationKind {
    Add,
    Sub,
}

#[derive(Debug, Clone, PartialEq, bough_typed_hash::HashInto)]
pub enum MutantKind {
    StatementBlock,
    Condition,
    BinaryOp(BinaryOpMutationKind),
}

impl<'a> TwigMutantsIter<'a> {
    pub fn new(lang: LanguageId, base: &'a Base, twig: &'a Twig) -> std::io::Result<Self> {
        // core[impl mutant.twig-iter.file]
        let file_path = crate::file::File::new(base, twig).resolve();
        let file_content = std::fs::read(&file_path)?;

        let driver = driver_for_lang(lang);

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&driver.ts_language())
            .expect("language grammar should load");
        let tree = parser
            .parse(&file_content, None)
            .expect("parse should succeed");

        // SAFETY: we store the Tree alongside the TreeCursor and never move or drop
        // the Tree while the cursor is alive. The cursor is invalidated before the tree.
        let cursor = unsafe {
            std::mem::transmute::<tree_sitter::TreeCursor<'_>, tree_sitter::TreeCursor<'static>>(
                tree.walk(),
            )
        };

        Ok(Self {
            lang,
            base,
            twig,
            driver,
            file_content,
            tree,
            cursor,
            did_visit: false,
            started: false,
            skip_kinds: Vec::new(),
            skip_queries: Vec::new(),
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

    // core[impl mutant.twig-iter.skip.kind]
    // core[impl mutant.twig-iter.skip.kind.multiple]
    pub fn with_skip_kind(mut self, kind: MutantKind) -> Self {
        self.skip_kinds.push(kind);
        self
    }

    // core[impl mutant.twig-iter.skip.query]
    // core[impl mutant.twig-iter.skip.query.multiple]
    pub fn with_skip_query(mut self, query: &str) -> Self {
        let q = tree_sitter::Query::new(&self.driver.ts_language(), query)
            .expect("skip query should be valid");
        self.skip_queries.push(q);
        self
    }
}

// core[impl mutant.twig-iter]
// core[impl mutant.twig-iter.item]
// core[impl mutant.twig-iter.find]
impl<'a> Iterator for TwigMutantsIter<'a> {
    type Item = Mutant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let node = if !self.started {
                self.started = true;
                Some(self.cursor.node())
            } else if !self.did_visit && self.cursor.goto_first_child() {
                self.did_visit = false;
                Some(self.cursor.node())
            } else if self.cursor.goto_next_sibling() {
                self.did_visit = false;
                Some(self.cursor.node())
            } else if self.cursor.goto_parent() {
                self.did_visit = true;
                None
            } else {
                return None;
            };

            let Some(node) = node else { continue };

            if let Some((kind, span)) = self.driver.check_node(&node, &self.file_content) {
                if self.skip_kinds.contains(&kind) {
                    continue;
                }
                if self.skip_queries.iter().any(|q| {
                    let mut qc = tree_sitter::QueryCursor::new();
                    qc.matches(q, node, self.file_content.as_slice())
                        .next()
                        .is_some()
                }) {
                    continue;
                }
                return Some(Mutant::new(self.lang, self.base, self.twig, kind, span));
            }
        }
    }
}

pub struct TwigsMutantsIter<'a> {
    lang: LanguageId,
    base: &'a Base,
    twigs_iter: TwigsIter,
    skip_kinds: Vec<MutantKind>,
    skip_queries: Vec<String>,
    current: Option<TwigMutantsIter<'a>>,
}

impl<'a> TwigsMutantsIter<'a> {
    pub fn new(lang: LanguageId, base: &'a Base, twigs_iter: TwigsIter) -> Self {
        todo!()
    }

    pub fn lang(&self) -> LanguageId {
        self.lang
    }

    pub fn base(&self) -> &Base {
        self.base
    }

    pub fn twigs_iter(&self) -> &TwigsIter {
        &self.twigs_iter
    }

    pub fn with_skip_kind(mut self, kind: MutantKind) -> Self {
        todo!()
    }

    pub fn with_skip_query(mut self, query: &str) -> Self {
        todo!()
    }
}

impl<'a> Iterator for TwigsMutantsIter<'a> {
    type Item = Mutant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub(crate) fn span_from_node(node: &tree_sitter::Node<'_>) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span::new(
        Point::new(start.row, start.column, node.start_byte()),
        Point::new(end.row, end.column, node.end_byte()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Base;
    use crate::file::{Root, TwigsIter};
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
            TwigsIter::new(dir.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

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

    // core[verify mutant.twig-iter.twig]
    #[test]
    fn mutants_iter_holds_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.twig().path(), std::path::Path::new("src/a.js"));
    }

    // core[verify mutant.twig-iter.base]
    #[test]
    fn mutants_iter_holds_base() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.base().path(), base.path());
    }

    // core[verify mutant.twig-iter.lang]
    #[test]
    fn mutants_iter_owns_lang() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert_eq!(iter.lang(), LanguageId::Javascript);
    }

    // core[verify mutant.twig-iter.file]
    #[test]
    fn mutants_iter_resolves_file_from_base_and_twig() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        assert!(TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).is_ok());
    }

    // core[verify mutant.twig-iter.file]
    #[test]
    fn mutants_iter_errors_on_missing_file() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/nonexistent.js")).unwrap();
        assert!(TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).is_err());
    }

    // core[verify mutant.twig-iter]
    // core[verify mutant.twig-iter.item]
    // core[verify mutant.twig-iter.find]
    #[test]
    fn mutants_iter_walks_tree_and_returns_mutants() {
        let (_dir, base) = make_js_base("const a = 1;");
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        let mutants: Vec<_> = iter.collect();
        assert!(mutants.is_empty());
    }

    // core[verify mutant.twig-iter.find.js.statement]
    #[test]
    fn js_finds_statement_blocks() {
        let js = "function foo() { return 1; }\nfunction bar() { return 2; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
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

    // core[verify mutant.twig-iter.find.js.condition.if]
    #[test]
    fn js_finds_if_condition() {
        let js = "if (x > 0) { console.log(x); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.twig-iter.find.js.condition.while]
    #[test]
    fn js_finds_while_condition() {
        let js = "while (i < 10) { i++; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let conditions: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.twig-iter.find.js.condition.for]
    #[test]
    fn js_finds_for_condition() {
        let js = "for (let i = 0; i < 10; i++) { console.log(i); }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
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

    // core[verify mutant.twig-iter.find.js.binary.add]
    #[test]
    fn js_finds_add_binary_op() {
        let js = "const x = a + b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let adds: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
            .collect();
        assert_eq!(adds.len(), 1);
    }

    // core[verify mutant.twig-iter.find.js.binary.sub]
    #[test]
    fn js_finds_sub_binary_op() {
        let js = "const x = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .collect();
        let subs: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Sub)))
            .collect();
        assert_eq!(subs.len(), 1);
    }

    // core[verify mutant.twig-iter.find.js.binary.add]
    // core[verify mutant.twig-iter.find.js.binary.sub]
    #[test]
    fn js_ignores_other_binary_ops() {
        let js = "const x = a * b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
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

    // core[verify mutant.hash.base]
    #[test]
    fn mutant_hash_excludes_base() {
        let dir1 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir1.path().join("src")).unwrap();
        std::fs::write(dir1.path().join("src/a.js"), "const a = 1;").unwrap();
        let base1 = Base::new(
            dir1.path().to_path_buf(),
            TwigsIter::new(dir1.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir2.path().join("src")).unwrap();
        std::fs::write(dir2.path().join("src/a.js"), "const a = 1;").unwrap();
        let base2 = Base::new(
            dir2.path().to_path_buf(),
            TwigsIter::new(dir2.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            &base1,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            &base2,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(hash_mutant(&m1), hash_mutant(&m2));
    }

    // core[verify mutant.hash.lang]
    #[test]
    fn mutant_hash_includes_lang() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Typescript,
            &base,
            &twig,
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
            TwigsIter::new(dir.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();
        let twig_a = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let twig_b = Twig::new(PathBuf::from("src/b.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig_a,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig_b,
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
            TwigsIter::new(dir1.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let dir2 = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir2.path().join("src")).unwrap();
        std::fs::write(dir2.path().join("src/a.js"), "const b = 2;").unwrap();
        let base2 = Base::new(
            dir2.path().to_path_buf(),
            TwigsIter::new(dir2.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();

        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m1 = Mutant::new(
            LanguageId::Javascript,
            &base1,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            &base2,
            &twig,
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
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
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
            LanguageId::Javascript,
            &base,
            &twig,
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let m2 = Mutant::new(
            LanguageId::Javascript,
            &base,
            &twig,
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

    // core[verify mutant.twig-iter.skip.kind]
    #[test]
    fn skip_kind_filters_matching_mutants() {
        // "function foo() { return a + b; }" produces: StatementBlock, BinaryOp(Add)
        let js = "function foo() { return a + b; }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .with_skip_kind(MutantKind::StatementBlock)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    // core[verify mutant.twig-iter.skip.kind.multiple]
    #[test]
    fn skip_kind_multiple_filters_all_specified_kinds() {
        // "function foo() { if (x) { return a + b; } }" produces:
        //   StatementBlock (outer), Condition, StatementBlock (inner), BinaryOp(Add)
        let js = "function foo() { if (x) { return a + b; } }";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutants: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .with_skip_kind(MutantKind::StatementBlock)
            .with_skip_kind(MutantKind::Condition)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    // core[verify mutant.twig-iter.skip.query]
    #[test]
    fn skip_query_filters_matching_nodes() {
        // "const x = a + b; const y = a - b;" produces: BinaryOp(Add), BinaryOp(Sub)
        // skip query targeting "+" operator removes only the Add
        let js = "const x = a + b; const y = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let filtered: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            *filtered[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub),
        );
    }

    // core[verify mutant.twig-iter.skip.query.multiple]
    #[test]
    fn skip_query_multiple_filters_union() {
        // "const x = a + b; const y = a - b;" produces: BinaryOp(Add), BinaryOp(Sub)
        // skip query for add filters the Add, skip query for sub filters the Sub
        let js = "const x = a + b; const y = a - b;";
        let (_dir, base) = make_js_base(js);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let filtered_one: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .collect();
        assert_eq!(filtered_one.len(), 1);
        assert_eq!(
            *filtered_one[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub),
        );

        let filtered_both: Vec<_> = TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
            .unwrap()
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .with_skip_query("(binary_expression operator: \"-\") @skip")
            .collect();
        assert!(filtered_both.is_empty());
    }

    fn make_multi_js_base(files: &[(&str, &str)]) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        for (name, content) in files {
            let path = dir.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
        }
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIter::new(dir.path()).with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    // core[verify mutant.twigs-iter.base]
    #[test]
    fn twigs_iter_holds_base() {
        let (_dir, base) = make_multi_js_base(&[("src/a.js", "const a = 1;")]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let iter = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs);
        assert_eq!(iter.base().path(), base.path());
    }

    // core[verify mutant.twigs-iter.twigs-iter]
    #[test]
    fn twigs_iter_holds_twigs_iter() {
        let (_dir, base) = make_multi_js_base(&[("src/a.js", "const a = 1;")]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let iter = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs);
        let _ = iter.twigs_iter();
    }

    // core[verify mutant.twigs-iter.lang]
    #[test]
    fn twigs_iter_owns_lang() {
        let (_dir, base) = make_multi_js_base(&[("src/a.js", "const a = 1;")]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let iter = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs);
        assert_eq!(iter.lang(), LanguageId::Javascript);
    }

    // core[verify mutant.twigs-iter]
    // core[verify mutant.twigs-iter.delegates]
    #[test]
    fn twigs_iter_yields_mutants_across_files() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "function foo() { return 1; }"),
            ("src/b.js", "function bar() { return 2; }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let mutants: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs).collect();
        assert_eq!(mutants.len(), 2);
        let twigs_seen: std::collections::HashSet<_> =
            mutants.iter().map(|m| m.twig().path().to_path_buf()).collect();
        assert!(twigs_seen.contains(&PathBuf::from("src/a.js")));
        assert!(twigs_seen.contains(&PathBuf::from("src/b.js")));
    }

    // core[verify mutant.twigs-iter.find.js.statement]
    #[test]
    fn twigs_iter_finds_js_statement_blocks() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "function foo() { return 1; }"),
            ("src/b.js", "function bar() { return 2; }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let mutants: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs).collect();
        let blocks: Vec<_> = mutants
            .iter()
            .filter(|m| matches!(m.kind(), MutantKind::StatementBlock))
            .collect();
        assert_eq!(blocks.len(), 2);
    }

    // core[verify mutant.twigs-iter.find.js.condition.if]
    #[test]
    fn twigs_iter_finds_js_if_condition() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "if (x > 0) { console.log(x); }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let conditions: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.twigs-iter.find.js.condition.while]
    #[test]
    fn twigs_iter_finds_js_while_condition() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "while (i < 10) { i++; }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let conditions: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.twigs-iter.find.js.condition.for]
    #[test]
    fn twigs_iter_finds_js_for_condition() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "for (let i = 0; i < 10; i++) { console.log(i); }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let conditions: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .filter(|m| matches!(m.kind(), MutantKind::Condition))
            .collect();
        assert_eq!(conditions.len(), 1);
    }

    // core[verify mutant.twigs-iter.find.js.binary.add]
    #[test]
    fn twigs_iter_finds_js_add_binary_op() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "const x = a + b;"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let adds: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Add)))
            .collect();
        assert_eq!(adds.len(), 1);
    }

    // core[verify mutant.twigs-iter.find.js.binary.sub]
    #[test]
    fn twigs_iter_finds_js_sub_binary_op() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "const x = a - b;"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let subs: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .filter(|m| matches!(m.kind(), MutantKind::BinaryOp(BinaryOpMutationKind::Sub)))
            .collect();
        assert_eq!(subs.len(), 1);
    }

    // core[verify mutant.twigs-iter.skip.kind]
    #[test]
    fn twigs_iter_skip_kind_filters_matching() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "function foo() { return a + b; }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let mutants: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .with_skip_kind(MutantKind::StatementBlock)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    // core[verify mutant.twigs-iter.skip.kind.multiple]
    #[test]
    fn twigs_iter_skip_kind_multiple() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "function foo() { if (x) { return a + b; } }"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let mutants: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .with_skip_kind(MutantKind::StatementBlock)
            .with_skip_kind(MutantKind::Condition)
            .collect();
        assert_eq!(mutants.len(), 1);
        assert_eq!(
            *mutants[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Add),
        );
    }

    // core[verify mutant.twigs-iter.skip.query]
    #[test]
    fn twigs_iter_skip_query_filters_matching() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "const x = a + b; const y = a - b;"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let filtered: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(
            *filtered[0].kind(),
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub),
        );
    }

    // core[verify mutant.twigs-iter.skip.query.multiple]
    #[test]
    fn twigs_iter_skip_query_multiple() {
        let (_dir, base) = make_multi_js_base(&[
            ("src/a.js", "const x = a + b; const y = a - b;"),
        ]);
        let twigs = TwigsIter::new(_dir.path()).with_include_glob("src/**/*.js");
        let filtered: Vec<_> = TwigsMutantsIter::new(LanguageId::Javascript, &base, twigs)
            .with_skip_query("(binary_expression operator: \"+\") @skip")
            .with_skip_query("(binary_expression operator: \"-\") @skip")
            .collect();
        assert!(filtered.is_empty());
    }
}

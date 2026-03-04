use crate::{LanguageId, base::Base, file::Twig};

trait LanguageDriver {
    fn check_node(
        &self,
        node: &tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)>;
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
    tree: tree_sitter::Tree,
    file_content: Vec<u8>,
    cursor_stack: Vec<usize>,
    driver: Box<dyn LanguageDriver>,
}

// core[impl mutant.lang]
// core[impl mutant.base]
// core[impl mutant.twig]
// core[impl mutant.kind]
// core[impl mutant.span]
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

// core[impl span.point]
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
pub struct Point {
    line: usize,
    col: usize,
    byte: usize,
}

pub enum BinaryOpMutationKind {
    Add,
    Sub,
}

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

        let mut parser = tree_sitter::Parser::new();
        let ts_lang = match lang {
            LanguageId::Javascript => tree_sitter_javascript::LANGUAGE,
            LanguageId::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
        };
        parser.set_language(&ts_lang.into()).expect("language grammar should load");
        let tree = parser.parse(&file_content, None).expect("parse should succeed");

        let driver: Box<dyn LanguageDriver> = match lang {
            LanguageId::Javascript => Box::new(JavascriptDriver),
            LanguageId::Typescript => Box::new(TypescriptDriver),
        };

        Ok(Self {
            lang,
            base,
            twig,
            tree,
            file_content,
            cursor_stack: vec![0],
            driver,
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
impl<'a> Iterator for MutantsIter<'a> {
    type Item = Mutant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl LanguageDriver for JavascriptDriver {
    fn check_node(
        &self,
        _node: &tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        todo!()
    }
}

impl LanguageDriver for TypescriptDriver {
    fn check_node(
        &self,
        _node: &tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        todo!()
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
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), "const a = 1;").unwrap();
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
    fn mutants_iter_resolves_file_path() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let iter = MutantsIter::new(LanguageId::Javascript, &base, &twig).unwrap();
        assert!(!iter.file_content.is_empty());
        assert_eq!(std::str::from_utf8(&iter.file_content).unwrap(), "const a = 1;");
    }

    // core[verify mutant.iter.file]
    #[test]
    fn mutants_iter_errors_on_missing_file() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/nonexistent.js")).unwrap();
        assert!(MutantsIter::new(LanguageId::Javascript, &base, &twig).is_err());
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

use crate::{LanguageId, base::Base, file::Twig};

trait LanguageDriver {
    fn next_mutant(
        &self,
        node: tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)>;
}

struct JavascriptDriver {}
struct TypescriptDriver {}

struct MutantsIter<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    current_node: tree_sitter::Node<'a>,
    driver: Box<dyn LanguageDriver>,
}

struct Mutant<'a> {
    lang: LanguageId,
    base: &'a Base,
    twig: &'a Twig,
    kind: MutantKind,
    span: Span,
}

struct Span {
    start: Point,
    end: Point,
}

// core[impl point.line]
// core[impl point.col]
// core[impl point.byte]
pub struct Point {
    line: usize,
    col: usize,
    byte: usize,
}

enum BinaryOpMutationKind {
    Add,
    Sub,
}

enum MutantKind {
    StatementBlock,
    Condition,
    BinaryOp(BinaryOpMutationKind),
}

impl<'a> MutantsIter<'a> {
    fn new(lang: LanguageId, base: &'a Base, twig: &'a Twig) -> Self {
        Self {
            lang,
            base,
            twig,
            driver: (match lang {
                LanguageId::Javascript => Box::new(JavascriptDriver::new()),
                LanguageId::Typescript => Box::new(TypescriptDriver::new()),
            }),
            current_node: todo!(),
        }
    }
}

impl<'a> Iterator for MutantsIter<'a> {
    type Item = Mutant<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

impl JavascriptDriver {
    fn new() -> Self {
        Self {}
    }
}
impl TypescriptDriver {
    fn new() -> Self {
        Self {}
    }
}
impl LanguageDriver for JavascriptDriver {
    fn next_mutant(
        &self,
        node: tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        todo!()
    }
}

impl LanguageDriver for TypescriptDriver {
    fn next_mutant(
        &self,
        node: tree_sitter::Node<'_>,
        file_content: &[u8],
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
}

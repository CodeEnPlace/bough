use crate::language::LanguageDriver;
use crate::LanguageId;
use arborium_tree_sitter::StreamingIterator;
use bough_fs::Twig;
use bough_typed_hash::{HashInto, TypedHashable};
use tracing::{debug, trace};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncompassError {
    LangMismatch {
        outer: LanguageId,
        inner: LanguageId,
    },
    TwigMismatch {
        outer: Twig,
        inner: Twig,
    },
}

pub struct TwigMutantsIter<'a, 't, R: bough_fs::Root> {
    lang: LanguageId,
    base: &'a R,
    twig: &'t Twig,
    driver: Box<dyn LanguageDriver>,
    file_content: Vec<u8>,
    tree: arborium_tree_sitter::Tree,
    cursor: arborium_tree_sitter::TreeCursor<'static>,
    did_visit: bool,
    started: bool,
    skip_kinds: Vec<MutantKind>,
    skip_queries: Vec<arborium_tree_sitter::Query>,
}

#[derive(bough_typed_hash::TypedHash)]
pub struct MutantHash([u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Hash, facet::Facet)]
pub struct Mutant {
    lang: LanguageId,
    twig: Twig,
    kind: MutantKind,
    subst_span: Span,
    effect_span: Span,
}

impl Mutant {
    pub fn new(
        lang: LanguageId,
        twig: Twig,
        kind: MutantKind,
        span: Span,
        effect_span: Span,
    ) -> Self {
        Self {
            lang,
            twig,
            kind,
            subst_span: span,
            effect_span,
        }
    }

    pub fn lang(&self) -> LanguageId {
        self.lang
    }

    pub fn twig(&self) -> &Twig {
        &self.twig
    }

    pub fn kind(&self) -> &MutantKind {
        &self.kind
    }

    pub fn span(&self) -> &Span {
        &self.subst_span
    }

    pub fn effect_span(&self) -> &Span {
        &self.effect_span
    }

    pub fn encompasses(&self, inner: &Mutant) -> Result<bool, EncompassError> {
        if self.lang != inner.lang {
            return Err(EncompassError::LangMismatch {
                outer: self.lang,
                inner: inner.lang,
            });
        }
        if self.twig != inner.twig {
            return Err(EncompassError::TwigMismatch {
                outer: self.twig.clone(),
                inner: inner.twig.clone(),
            });
        }
        Ok(self.effect_span.contains(&inner.subst_span))
    }

    pub fn get_contextual_fragment(
        &self,
        base: &impl bough_fs::Root,
        context_lines: usize,
        driver: &dyn LanguageDriver,
    ) -> Result<(String, Span), std::io::Error> {
        let file_path = bough_fs::File::new(base, &self.twig).resolve();
        let file_content = std::fs::read(&file_path)?;
        let mut parser = arborium_tree_sitter::Parser::new();
        parser
            .set_language(&driver.ts_language())
            .expect("language grammar should load");
        let tree = parser
            .parse(&file_content, None)
            .expect("parse should succeed");

        let root = tree.root_node();
        let target = root
            .descendant_for_byte_range(self.subst_span.start().byte(), self.subst_span.end().byte())
            .unwrap_or(root);

        let mutant_start_line = self.subst_span.start().line();
        let mutant_end_line = self.subst_span.end().line();

        let mut candidate = target;
        let mut current = target;

        let has_context = |node: &arborium_tree_sitter::Node<'_>| -> bool {
            let above = mutant_start_line.saturating_sub(node.start_position().row);
            let below = node.end_position().row.saturating_sub(mutant_end_line);
            above >= context_lines && below >= context_lines
        };

        let line_range = |node: &arborium_tree_sitter::Node<'_>| -> (usize, usize) {
            (node.start_position().row, node.end_position().row)
        };

        let mut context_met = has_context(&current);

        loop {
            if driver.is_context_boundary(&current) {
                candidate = current;
                break;
            }

            if context_met {
                let candidate_range = line_range(&candidate);
                let current_range = line_range(&current);
                if current_range != candidate_range {
                    break;
                }
                candidate = current;
            } else if has_context(&current) {
                context_met = true;
                candidate = current;
            }

            if let Some(parent) = current.parent() {
                current = parent;
            } else {
                candidate = current;
                break;
            }
        }

        let text = std::str::from_utf8(&file_content[candidate.start_byte()..candidate.end_byte()])
            .unwrap_or("")
            .to_string();

        Ok((text, span_from_node(&candidate)))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BasedMutant<'a, R: bough_fs::Root> {
    mutant: Mutant,
    base: &'a R,
}

impl<'a, R: bough_fs::Root> BasedMutant<'a, R> {
    pub fn new(mutant: Mutant, base: &'a R) -> Self {
        Self { mutant, base }
    }

    pub fn mutant(&self) -> &Mutant {
        &self.mutant
    }

    pub fn into_mutant(self) -> Mutant {
        self.mutant
    }

    pub fn base(&self) -> &R {
        self.base
    }

    pub fn lang(&self) -> LanguageId {
        self.mutant.lang
    }

    pub fn twig(&self) -> &Twig {
        &self.mutant.twig
    }

    pub fn kind(&self) -> &MutantKind {
        &self.mutant.kind
    }

    pub fn span(&self) -> &Span {
        &self.mutant.subst_span
    }

    pub fn effect_span(&self) -> &Span {
        &self.mutant.effect_span
    }
}

impl HashInto for Mutant {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.lang.hash_into(state)?;
        self.twig.as_str().as_bytes().hash_into(state)?;
        self.subst_span.hash_into(state)?;
        self.kind.hash_into(state)?;
        Ok(())
    }
}

impl TypedHashable for Mutant {
    type Hash = MutantHash;
}

#[derive(bough_typed_hash::TypedHash)]
pub struct BasedMutantHash([u8; 32]);

impl<R: bough_fs::Root> HashInto for BasedMutant<'_, R> {
    fn hash_into(&self, state: &mut bough_typed_hash::ShaState) -> Result<(), std::io::Error> {
        self.mutant.hash_into(state)?;
        bough_fs::File::new(self.base, &self.mutant.twig).hash_into(state)?;
        Ok(())
    }
}

impl<R: bough_fs::Root> TypedHashable for BasedMutant<'_, R> {
    type Hash = BasedMutantHash;
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
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

    pub fn intersects(&self, other: &Span) -> bool {
        self.start.byte < other.end.byte && other.start.byte < self.end.byte
    }

    pub fn contains(&self, inner: &Span) -> bool {
        self.start.byte <= inner.start.byte && inner.end.byte <= self.end.byte
    }
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
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

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum BinaryOpMutationKind {
    Add,
    And,
    BitAnd,
    BitOr,
    BitXor,
    Div,
    Eq,
    Exp,
    FloorDiv,
    Gt,
    In,
    Is,
    IsNot,
    NotIn,
    Gte,
    Lt,
    Lte,
    Mul,
    Or,
    Rem,
    Shl,
    Shr,
    StrictEq,
    Ne,
    StrictNe,
    Sub,
    Xor,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum AssignMutationKind {
    AddAssign,
    AndAssign,
    BitAndAssign,
    BitOrAssign,
    BitXorAssign,
    DivAssign,
    ExpAssign,
    FloorDivAssign,
    MulAssign,
    NormalAssign,
    OrAssign,
    RemAssign,
    ShlAssign,
    ShrAssign,
    SubAssign,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum ArrayDeclKind {
    Inline,
    Instance,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum LiteralKind {
    BoolTrue,
    BoolFalse,
    String,
    EmptyString,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum OptionalChainKind {
    Literal,
    Indexed,
    FnCall,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum RangeKind {
    Exclusive,
    Inclusive,
    From,
}

#[derive(Debug, Clone, PartialEq, Eq, bough_typed_hash::HashInto, Hash, facet::Facet)]
#[repr(u8)]
pub enum MutantKind {
    StatementBlock,
    Condition,
    BinaryOp(BinaryOpMutationKind),
    Assign(AssignMutationKind),
    ArrayDecl(ArrayDeclKind),
    Literal(LiteralKind),
    DictDecl,
    TupleDecl,
    Assert,
    UnaryNot,
    OptionalChain(OptionalChainKind),
    SwitchCase,
    Range(RangeKind),
    MatchPattern,
}

impl MutantKind {
    pub fn all_variants() -> Vec<MutantKind> {
        use ArrayDeclKind::*;
        use AssignMutationKind::*;
        use BinaryOpMutationKind::*;
        use LiteralKind::*;
        use OptionalChainKind::*;

        vec![
            MutantKind::StatementBlock,
            MutantKind::Condition,
            MutantKind::BinaryOp(Add),
            MutantKind::BinaryOp(And),
            MutantKind::BinaryOp(BitAnd),
            MutantKind::BinaryOp(BitOr),
            MutantKind::BinaryOp(BitXor),
            MutantKind::BinaryOp(Div),
            MutantKind::BinaryOp(Eq),
            MutantKind::BinaryOp(Exp),
            MutantKind::BinaryOp(FloorDiv),
            MutantKind::BinaryOp(Gt),
            MutantKind::BinaryOp(In),
            MutantKind::BinaryOp(Is),
            MutantKind::BinaryOp(IsNot),
            MutantKind::BinaryOp(NotIn),
            MutantKind::BinaryOp(Gte),
            MutantKind::BinaryOp(Lt),
            MutantKind::BinaryOp(Lte),
            MutantKind::BinaryOp(Mul),
            MutantKind::BinaryOp(Or),
            MutantKind::BinaryOp(Rem),
            MutantKind::BinaryOp(Shl),
            MutantKind::BinaryOp(Shr),
            MutantKind::BinaryOp(StrictEq),
            MutantKind::BinaryOp(Ne),
            MutantKind::BinaryOp(StrictNe),
            MutantKind::BinaryOp(Sub),
            MutantKind::BinaryOp(Xor),
            MutantKind::Assign(AddAssign),
            MutantKind::Assign(AndAssign),
            MutantKind::Assign(BitAndAssign),
            MutantKind::Assign(BitOrAssign),
            MutantKind::Assign(BitXorAssign),
            MutantKind::Assign(DivAssign),
            MutantKind::Assign(ExpAssign),
            MutantKind::Assign(FloorDivAssign),
            MutantKind::Assign(MulAssign),
            MutantKind::Assign(NormalAssign),
            MutantKind::Assign(OrAssign),
            MutantKind::Assign(RemAssign),
            MutantKind::Assign(ShlAssign),
            MutantKind::Assign(ShrAssign),
            MutantKind::Assign(SubAssign),
            MutantKind::ArrayDecl(Inline),
            MutantKind::ArrayDecl(Instance),
            MutantKind::Literal(BoolTrue),
            MutantKind::Literal(BoolFalse),
            MutantKind::Literal(String),
            MutantKind::Literal(EmptyString),
            MutantKind::Literal(Number),
            MutantKind::DictDecl,
            MutantKind::TupleDecl,
            MutantKind::Assert,
            MutantKind::UnaryNot,
            MutantKind::OptionalChain(Literal),
            MutantKind::OptionalChain(Indexed),
            MutantKind::OptionalChain(FnCall),
            MutantKind::SwitchCase,
            MutantKind::Range(RangeKind::Exclusive),
            MutantKind::Range(RangeKind::Inclusive),
            MutantKind::Range(RangeKind::From),
            MutantKind::MatchPattern,
        ]
    }

    pub fn heading(&self) -> std::string::String {
        match self {
            MutantKind::StatementBlock => "StatementBlock".into(),
            MutantKind::Condition => "Condition".into(),
            MutantKind::BinaryOp(k) => format!("BinaryOp - {k:?}"),
            MutantKind::Assign(k) => format!("Assign - {k:?}"),
            MutantKind::ArrayDecl(k) => format!("ArrayDecl - {k:?}"),
            MutantKind::Literal(k) => format!("Literal - {k:?}"),
            MutantKind::DictDecl => "DictDecl".into(),
            MutantKind::TupleDecl => "TupleDecl".into(),
            MutantKind::Assert => "Assert".into(),
            MutantKind::UnaryNot => "UnaryNot".into(),
            MutantKind::OptionalChain(k) => format!("OptionalChain - {k:?}"),
            MutantKind::SwitchCase => "SwitchCase".into(),
            MutantKind::Range(k) => format!("Range - {k:?}"),
            MutantKind::MatchPattern => "MatchPattern".into(),
        }
    }

    /// Serialise this kind to a stable string key.
    ///
    /// Simple kinds produce their variant name (e.g. `"StatementBlock"`).
    /// Parameterised kinds use Rust-enum-style parens (e.g. `"BinaryOp(Add)"`).
    pub fn to_key(&self) -> String {
        match self {
            MutantKind::StatementBlock => "StatementBlock".into(),
            MutantKind::Condition => "Condition".into(),
            MutantKind::BinaryOp(k) => format!("BinaryOp({k:?})"),
            MutantKind::Assign(k) => format!("Assign({k:?})"),
            MutantKind::ArrayDecl(k) => format!("ArrayDecl({k:?})"),
            MutantKind::Literal(k) => format!("Literal({k:?})"),
            MutantKind::DictDecl => "DictDecl".into(),
            MutantKind::TupleDecl => "TupleDecl".into(),
            MutantKind::Assert => "Assert".into(),
            MutantKind::UnaryNot => "UnaryNot".into(),
            MutantKind::OptionalChain(k) => format!("OptionalChain({k:?})"),
            MutantKind::SwitchCase => "SwitchCase".into(),
            MutantKind::Range(k) => format!("Range({k:?})"),
            MutantKind::MatchPattern => "MatchPattern".into(),
        }
    }

    /// Parse a key string (as produced by `to_key`) back into a `MutantKind`.
    ///
    /// Returns `None` if the key doesn't match any known kind.
    pub fn from_key(key: &str) -> Option<MutantKind> {
        // Simple (no-payload) kinds
        match key {
            "StatementBlock" => return Some(MutantKind::StatementBlock),
            "Condition" => return Some(MutantKind::Condition),
            "DictDecl" => return Some(MutantKind::DictDecl),
            "TupleDecl" => return Some(MutantKind::TupleDecl),
            "Assert" => return Some(MutantKind::Assert),
            "UnaryNot" => return Some(MutantKind::UnaryNot),
            "SwitchCase" => return Some(MutantKind::SwitchCase),
            "MatchPattern" => return Some(MutantKind::MatchPattern),
            _ => {}
        }

        // Parameterised kinds: "Outer(Inner)"
        let open = key.find('(')?;
        let close = key.strip_suffix(')')?;
        let outer = &close[..open];
        let inner = &close[open + 1..];

        match outer {
            "BinaryOp" => {
                use BinaryOpMutationKind::*;
                let k = match inner {
                    "Add" => Add,
                    "And" => And,
                    "BitAnd" => BitAnd,
                    "BitOr" => BitOr,
                    "BitXor" => BitXor,
                    "Div" => Div,
                    "Eq" => Eq,
                    "Exp" => Exp,
                    "FloorDiv" => FloorDiv,
                    "Gt" => Gt,
                    "In" => In,
                    "Is" => Is,
                    "IsNot" => IsNot,
                    "NotIn" => NotIn,
                    "Gte" => Gte,
                    "Lt" => Lt,
                    "Lte" => Lte,
                    "Mul" => Mul,
                    "Or" => Or,
                    "Rem" => Rem,
                    "Shl" => Shl,
                    "Shr" => Shr,
                    "StrictEq" => StrictEq,
                    "Ne" => Ne,
                    "StrictNe" => StrictNe,
                    "Sub" => Sub,
                    "Xor" => Xor,
                    _ => return None,
                };
                Some(MutantKind::BinaryOp(k))
            }
            "Assign" => {
                use AssignMutationKind::*;
                let k = match inner {
                    "AddAssign" => AddAssign,
                    "AndAssign" => AndAssign,
                    "BitAndAssign" => BitAndAssign,
                    "BitOrAssign" => BitOrAssign,
                    "BitXorAssign" => BitXorAssign,
                    "DivAssign" => DivAssign,
                    "ExpAssign" => ExpAssign,
                    "FloorDivAssign" => FloorDivAssign,
                    "MulAssign" => MulAssign,
                    "NormalAssign" => NormalAssign,
                    "OrAssign" => OrAssign,
                    "RemAssign" => RemAssign,
                    "ShlAssign" => ShlAssign,
                    "ShrAssign" => ShrAssign,
                    "SubAssign" => SubAssign,
                    _ => return None,
                };
                Some(MutantKind::Assign(k))
            }
            "ArrayDecl" => {
                let k = match inner {
                    "Inline" => ArrayDeclKind::Inline,
                    "Instance" => ArrayDeclKind::Instance,
                    _ => return None,
                };
                Some(MutantKind::ArrayDecl(k))
            }
            "Literal" => {
                use LiteralKind::*;
                let k = match inner {
                    "BoolTrue" => BoolTrue,
                    "BoolFalse" => BoolFalse,
                    "String" => String,
                    "EmptyString" => EmptyString,
                    "Number" => Number,
                    _ => return None,
                };
                Some(MutantKind::Literal(k))
            }
            "OptionalChain" => {
                let k = match inner {
                    "Literal" => OptionalChainKind::Literal,
                    "Indexed" => OptionalChainKind::Indexed,
                    "FnCall" => OptionalChainKind::FnCall,
                    _ => return None,
                };
                Some(MutantKind::OptionalChain(k))
            }
            "Range" => {
                let k = match inner {
                    "Exclusive" => RangeKind::Exclusive,
                    "Inclusive" => RangeKind::Inclusive,
                    "From" => RangeKind::From,
                    _ => return None,
                };
                Some(MutantKind::Range(k))
            }
            _ => None,
        }
    }
}

impl<'a, 't, R: bough_fs::Root> TwigMutantsIter<'a, 't, R> {
    pub fn new(lang: LanguageId, base: &'a R, twig: &'t Twig, driver: Box<dyn LanguageDriver>) -> std::io::Result<Self> {
        let file_path = bough_fs::File::new(base, twig).resolve();
        debug!(lang = ?lang, path = %file_path.display(), "parsing twig for mutants");
        let file_content = std::fs::read(&file_path)?;

        let mut parser = arborium_tree_sitter::Parser::new();
        parser
            .set_language(&driver.ts_language())
            .expect("language grammar should load");
        let tree = parser
            .parse(&file_content, None)
            .expect("parse should succeed");

        // SAFETY: we store the Tree alongside the TreeCursor and never move or drop
        // the Tree while the cursor is alive. The cursor is invalidated before the tree.
        let cursor = unsafe {
            std::mem::transmute::<
                arborium_tree_sitter::TreeCursor<'_>,
                arborium_tree_sitter::TreeCursor<'static>,
            >(tree.walk())
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

    pub fn base(&self) -> &R {
        self.base
    }

    pub fn twig(&self) -> &Twig {
        self.twig
    }

    pub fn with_skip_kind(mut self, kind: MutantKind) -> Self {
        debug!(?kind, "adding skip kind filter");
        self.skip_kinds.push(kind);
        self
    }

    pub fn with_skip_query(mut self, query: &str) -> Self {
        debug!(query, "adding skip query filter");
        let q = arborium_tree_sitter::Query::new(&self.driver.ts_language(), query)
            .expect("skip query should be valid");
        self.skip_queries.push(q);
        self
    }
}

#[allow(clippy::if_same_then_else)]
impl<'a, 't, R: bough_fs::Root> Iterator for TwigMutantsIter<'a, 't, R> {
    type Item = BasedMutant<'a, R>;

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

            if let Some((kind, span, effect_span)) =
                self.driver.check_node(&node, &self.file_content)
            {
                if self.skip_kinds.contains(&kind) {
                    trace!(?kind, "skipping mutant by kind filter");
                    continue;
                }
                if self.skip_queries.iter().any(|q| {
                    let mut qc = arborium_tree_sitter::QueryCursor::new();
                    let root = self.tree.root_node();
                    let skip_idx = q
                        .capture_names()
                        .iter()
                        .position(|n| *n == "skip")
                        .expect("skip query must have a @skip capture")
                        as u32;
                    qc.matches(q, root, self.file_content.as_slice()).any(|m| {
                        m.nodes_for_capture_index(skip_idx).any(|n| {
                            node.start_byte() >= n.start_byte() && node.end_byte() <= n.end_byte()
                        })
                    })
                }) {
                    trace!(?kind, "skipping mutant by query filter");
                    continue;
                }
                trace!(
                    ?kind,
                    start = span.start().byte(),
                    end = span.end().byte(),
                    "found mutant"
                );
                let mutant = Mutant::new(self.lang, self.twig.clone(), kind, span, effect_span);
                return Some(BasedMutant::new(mutant, self.base));
            }
        }
    }
}

pub fn span_from_node(node: &arborium_tree_sitter::Node<'_>) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span::new(
        Point::new(start.row, start.column, node.start_byte()),
        Point::new(end.row, end.column, node.end_byte()),
    )
}

/// A mutant found by parsing source code in-memory, without filesystem context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMutant {
    pub kind: MutantKind,
    pub subst_span: Span,
    pub effect_span: Span,
}

/// Parse source code in-memory and return all mutants found.
///
/// This uses the same tree-sitter walk and `LanguageDriver::check_node` logic
/// as `TwigMutantsIter`, but operates on a byte slice directly without
/// requiring a `Base` or `Twig`.
#[allow(clippy::if_same_then_else)]
pub fn find_mutants_in_source(driver: &dyn LanguageDriver, source: &[u8]) -> Vec<SourceMutant> {

    let mut parser = arborium_tree_sitter::Parser::new();
    parser
        .set_language(&driver.ts_language())
        .expect("language grammar should load");
    let tree = parser.parse(source, None).expect("parse should succeed");

    let mut cursor = tree.walk();
    let mut results = Vec::new();
    let mut did_visit = false;
    let mut started = false;

    loop {
        let node = if !started {
            started = true;
            Some(cursor.node())
        } else if !did_visit && cursor.goto_first_child() {
            did_visit = false;
            Some(cursor.node())
        } else if cursor.goto_next_sibling() {
            did_visit = false;
            Some(cursor.node())
        } else if cursor.goto_parent() {
            did_visit = true;
            None
        } else {
            break;
        };

        let Some(node) = node else { continue };

        if let Some((kind, subst_span, effect_span)) = driver.check_node(&node, source) {
            results.push(SourceMutant {
                kind,
                subst_span,
                effect_span,
            });
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use bough_fs::{Root, TestRoot};
    use std::path::PathBuf;

    fn make_base() -> (tempfile::TempDir, TestRoot) {
        make_js_base("const a = 1;")
    }

    fn make_js_base(content: &str) -> (tempfile::TempDir, TestRoot) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = TestRoot::new(dir.path());
        (dir, base)
    }

    #[test]
    fn point_line() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.line(), 10);
    }

    #[test]
    fn point_col() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.col(), 5);
    }

    #[test]
    fn point_byte() {
        let p = Point::new(10, 5, 42);
        assert_eq!(p.byte(), 42);
    }

    #[test]
    fn mutant_owns_language_id() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(m.lang(), LanguageId::Javascript);
    }

    #[test]
    fn mutant_holds_twig() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert_eq!(m.twig().path(), std::path::Path::new("src/a.js"));
    }

    #[test]
    fn mutant_owns_kind() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        assert!(matches!(m.kind(), MutantKind::Condition));
    }

    #[test]
    fn mutant_owns_span() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let m = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(3, 5, 30), Point::new(7, 1, 60)),
            Span::new(Point::new(3, 5, 30), Point::new(7, 1, 60)),
        );
        assert_eq!(m.span().start().line(), 3);
        assert_eq!(m.span().end().byte(), 60);
    }

    #[test]
    fn encompases_returns_true_when_effect_intersects_inner_subst() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let outer = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 4, 4), Point::new(0, 5, 5)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 20, 20)),
        );
        let inner = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 7, 7), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 7, 7), Point::new(0, 15, 15)),
        );
        assert!(outer.encompasses(&inner).unwrap());
    }

    #[test]
    fn encompases_returns_false_when_no_intersection() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let a = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(0, 5, 5)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
        );
        let b = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 20, 20), Point::new(0, 30, 30)),
            Span::new(Point::new(0, 20, 20), Point::new(0, 30, 30)),
        );
        assert!(!a.encompasses(&b).unwrap());
    }

    #[test]
    fn encompases_errors_on_different_twigs() {
        let a = Mutant::new(
            LanguageId::Javascript,
            Twig::new(PathBuf::from("src/a.js")).unwrap(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 20, 20)),
        );
        let b = Mutant::new(
            LanguageId::Javascript,
            Twig::new(PathBuf::from("src/b.js")).unwrap(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15)),
        );
        assert!(a.encompasses(&b).is_err());
    }

    #[test]
    fn encompases_errors_on_different_langs() {
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let a = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::Condition,
            Span::new(Point::new(0, 0, 0), Point::new(0, 10, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(0, 20, 20)),
        );
        let b = Mutant::new(
            LanguageId::Typescript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15)),
            Span::new(Point::new(0, 5, 5), Point::new(0, 15, 15)),
        );
        assert!(a.encompasses(&b).is_err());
    }

    #[test]
    fn based_mutant_holds_mutant() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let based = BasedMutant::new(mutant.clone(), &base);
        assert_eq!(based.mutant(), &mutant);
    }

    #[test]
    fn based_mutant_holds_base() {
        let (_dir, base) = make_base();
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        let mutant = Mutant::new(
            LanguageId::Javascript,
            twig.clone(),
            MutantKind::StatementBlock,
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
            Span::new(Point::new(0, 0, 0), Point::new(1, 0, 10)),
        );
        let based = BasedMutant::new(mutant, &base);
        assert_eq!(based.base().path(), base.path());
    }
}

use super::LanguageDriver;
use crate::mutant::{
    AssignMutationKind, BinaryOpMutationKind, LiteralKind, MutantKind, RangeKind, Span,
    span_from_node,
};
use tracing::trace;

pub(crate) struct RustDriver;

impl LanguageDriver for RustDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_rust::language().into()
    }

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        let result = match node.kind() {
            "binary_expression" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+" => BinaryOpMutationKind::Add,
                    "-" => BinaryOpMutationKind::Sub,
                    "*" => BinaryOpMutationKind::Mul,
                    "/" => BinaryOpMutationKind::Div,
                    "%" => BinaryOpMutationKind::Rem,
                    "&" => BinaryOpMutationKind::BitAnd,
                    "|" => BinaryOpMutationKind::BitOr,
                    "^" => BinaryOpMutationKind::BitXor,
                    "<<" => BinaryOpMutationKind::Shl,
                    ">>" => BinaryOpMutationKind::Shr,
                    "&&" => BinaryOpMutationKind::And,
                    "||" => BinaryOpMutationKind::Or,
                    "==" => BinaryOpMutationKind::Eq,
                    "!=" => BinaryOpMutationKind::Ne,
                    ">" => BinaryOpMutationKind::Gt,
                    ">=" => BinaryOpMutationKind::Gte,
                    "<" => BinaryOpMutationKind::Lt,
                    "<=" => BinaryOpMutationKind::Lte,
                    _ => return None,
                };
                Some((
                    MutantKind::BinaryOp(kind),
                    span_from_node(&op_node),
                    span_from_node(node),
                ))
            }
            "assignment_expression" => {
                // Workaround: tree-sitter-rust misparses `..=expr` as `.. = expr`.
                // See https://github.com/CodeEnPlace/bough/issues/38
                let left = node.child_by_field_name("left")?;
                if left.kind() == "range_expression" {
                    let has_left_operand = left.child(0).map(|c| c.kind() != "..").unwrap_or(false);
                    if !has_left_operand {
                        let eq_node = (0..node.child_count())
                            .filter_map(|i| node.child(i as u32))
                            .find(|c| c.kind() == "=")?;
                        let op_span = Span::new(
                            span_from_node(&left).start().clone(),
                            span_from_node(&eq_node).end().clone(),
                        );
                        return Some((
                            MutantKind::Range(RangeKind::Inclusive),
                            op_span,
                            span_from_node(node),
                        ));
                    }
                }
                let eq_node = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == "=")?;
                Some((
                    MutantKind::Assign(AssignMutationKind::NormalAssign),
                    span_from_node(&eq_node),
                    span_from_node(node),
                ))
            }
            "compound_assignment_expr" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+=" => AssignMutationKind::AddAssign,
                    "-=" => AssignMutationKind::SubAssign,
                    "*=" => AssignMutationKind::MulAssign,
                    "/=" => AssignMutationKind::DivAssign,
                    "%=" => AssignMutationKind::RemAssign,
                    "&=" => AssignMutationKind::BitAndAssign,
                    "|=" => AssignMutationKind::BitOrAssign,
                    "^=" => AssignMutationKind::BitXorAssign,
                    "<<=" => AssignMutationKind::ShlAssign,
                    ">>=" => AssignMutationKind::ShrAssign,
                    _ => return None,
                };
                Some((
                    MutantKind::Assign(kind),
                    span_from_node(&op_node),
                    span_from_node(node),
                ))
            }
            "range_expression" => {
                let op_node = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == ".." || c.kind() == "..=")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let first_child = node.child(0);
                let last_child = node.child(node.child_count().wrapping_sub(1) as u32);
                let has_left = first_child
                    .as_ref()
                    .map(|c| c.kind() != ".." && c.kind() != "..=")
                    .unwrap_or(false);
                let has_right = last_child
                    .as_ref()
                    .map(|c| c.kind() != ".." && c.kind() != "..=")
                    .unwrap_or(false);
                match op_text {
                    ".." if has_right => Some((
                        MutantKind::Range(RangeKind::Exclusive),
                        span_from_node(&op_node),
                        span_from_node(node),
                    )),
                    ".." if has_left => Some((
                        MutantKind::Range(RangeKind::From),
                        span_from_node(&first_child.unwrap()),
                        span_from_node(node),
                    )),
                    ".." => None,
                    "..=" => Some((
                        MutantKind::Range(RangeKind::Inclusive),
                        span_from_node(&op_node),
                        span_from_node(node),
                    )),
                    _ => None,
                }
            }
            "block" => {
                let span = span_from_node(node);
                Some((MutantKind::StatementBlock, span.clone(), span))
            }
            "if_expression" | "while_expression" => {
                let condition = node.child_by_field_name("condition")?;
                Some((
                    MutantKind::Condition,
                    span_from_node(&condition),
                    span_from_node(node),
                ))
            }
            "match_arm" => {
                let span = span_from_node(node);
                Some((MutantKind::SwitchCase, span.clone(), span))
            }
            "match_pattern" => {
                if let Some(condition) = node.child_by_field_name("condition") {
                    let arm = node.parent()?;
                    return Some((
                        MutantKind::Condition,
                        span_from_node(&condition),
                        span_from_node(&arm),
                    ));
                }
                let is_wildcard = node.child_count() == 1
                    && node.child(0).map(|c| c.kind() == "_").unwrap_or(false);
                if is_wildcard {
                    return None;
                }
                let pattern_span = span_from_node(node);
                let arm = node.parent()?;
                Some((MutantKind::MatchPattern, pattern_span, span_from_node(&arm)))
            }
            "integer_literal" | "float_literal" => {
                let span = span_from_node(node);
                Some((MutantKind::Literal(LiteralKind::Number), span.clone(), span))
            }
            "string_literal" => {
                let text = node.utf8_text(file_content).ok()?;
                let kind = if text == "\"\"" {
                    LiteralKind::EmptyString
                } else {
                    LiteralKind::String
                };
                let span = span_from_node(node);
                Some((MutantKind::Literal(kind), span.clone(), span))
            }
            "true" => {
                let span = span_from_node(node);
                Some((
                    MutantKind::Literal(LiteralKind::BoolTrue),
                    span.clone(),
                    span,
                ))
            }
            "false" => {
                let span = span_from_node(node);
                Some((
                    MutantKind::Literal(LiteralKind::BoolFalse),
                    span.clone(),
                    span,
                ))
            }
            "array_expression" => {
                if node.named_child_count() == 0 {
                    return None;
                }
                let span = span_from_node(node);
                Some((
                    MutantKind::ArrayDecl(crate::mutant::ArrayDeclKind::Inline),
                    span.clone(),
                    span,
                ))
            }
            "tuple_expression" => {
                if node.named_child_count() == 0 {
                    return None;
                }
                let span = span_from_node(node);
                Some((MutantKind::TupleDecl, span.clone(), span))
            }
            "unary_expression" => {
                let op_node = node.child(0)?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                if op_text == "!" {
                    Some((
                        MutantKind::UnaryNot,
                        span_from_node(&op_node),
                        span_from_node(node),
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((ref kind, ref span, _)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "rs: matched node");
        }
        result
    }

    fn substitutions(&self, kind: &MutantKind) -> Vec<String> {
        match kind {
            MutantKind::BinaryOp(BinaryOpMutationKind::Add) => vec!["-".into(), "*".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub) => vec!["+".into(), "/".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Mul) => vec!["/".into(), "+".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Div) => vec!["*".into(), "-".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Rem) => vec!["*".into(), "/".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitAnd) => vec!["|".into(), "^".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitOr) => vec!["&".into(), "^".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitXor) => vec!["&".into(), "|".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Shl) => vec![">>".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Shr) => vec!["<<".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::And) => vec!["||".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Or) => vec!["&&".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Eq) => vec!["!=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Ne) => vec!["==".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gt) => vec!["<=".into(), ">=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gte) => vec!["<".into(), ">".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lt) => vec![">=".into(), "<=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lte) => vec![">".into(), "<".into()],
            MutantKind::Assign(AssignMutationKind::NormalAssign) => {
                vec!["+=".into(), "-=".into()]
            }
            MutantKind::Assign(AssignMutationKind::AddAssign) => vec!["-=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::SubAssign) => vec!["+=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::MulAssign) => vec!["/=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::DivAssign) => vec!["*=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::RemAssign) => vec!["*=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::BitAndAssign) => vec!["|=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::BitOrAssign) => vec!["&=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::BitXorAssign) => vec!["&=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::ShlAssign) => vec![">>=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::ShrAssign) => vec!["<<=".into(), "=".into()],
            MutantKind::StatementBlock => vec!["{}".into()],
            MutantKind::Condition => vec!["true".into(), "false".into()],
            MutantKind::SwitchCase => vec!["".into()],
            MutantKind::Literal(LiteralKind::Number) => vec!["0".into(), "1".into(), "-1".into()],
            MutantKind::Literal(LiteralKind::String) => vec!["\"\"".into()],
            MutantKind::Literal(LiteralKind::EmptyString) => vec!["\"bough\"".into()],
            MutantKind::Literal(LiteralKind::BoolTrue) => vec!["false".into()],
            MutantKind::Literal(LiteralKind::BoolFalse) => vec!["true".into()],
            MutantKind::ArrayDecl(crate::mutant::ArrayDeclKind::Inline) => vec!["[]".into()],
            MutantKind::TupleDecl => vec!["()".into()],
            MutantKind::UnaryNot => vec!["".into()],
            MutantKind::Range(RangeKind::Exclusive) => vec!["..=".into()],
            MutantKind::Range(RangeKind::Inclusive) => vec!["..".into()],
            MutantKind::Range(RangeKind::From) => vec!["".into()],
            MutantKind::MatchPattern => vec!["_".into()],
            _ => vec![],
        }
    }

    fn is_context_boundary(&self, node: &arborium_tree_sitter::Node<'_>) -> bool {
        matches!(
            node.kind(),
            "function_item" | "impl_item" | "struct_item" | "enum_item" | "trait_item" | "mod_item"
        )
    }
}

#[cfg(test)]
mod tests {
    fn dump_tree(src: &str) {
        let lang: arborium_tree_sitter::Language = arborium_rust::language().into();
        let mut parser = arborium_tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(src.as_bytes(), None).unwrap();
        fn print_node(node: &arborium_tree_sitter::Node, src: &[u8], indent: usize) {
            let text = node.utf8_text(src).unwrap_or("");
            let field = node.parent().and_then(|p| {
                (0..p.child_count())
                    .find(|&i| {
                        p.child(i as u32)
                            .map(|c| c.id() == node.id())
                            .unwrap_or(false)
                    })
                    .and_then(|i| p.field_name_for_child(i as u32))
            });
            let field_str = field.map(|f| format!("{f}: ")).unwrap_or_default();
            eprintln!(
                "{:indent$}{field_str}{} [{}-{}] {text:?}",
                "",
                node.kind(),
                node.start_byte(),
                node.end_byte(),
                indent = indent
            );
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i as u32) {
                    print_node(&child, src, indent + 2);
                }
            }
        }
        print_node(&tree.root_node(), src.as_bytes(), 0);
    }

    #[test]
    #[ignore]
    fn debug_tree() {
        dump_tree("let x = a + b;");
    }
}

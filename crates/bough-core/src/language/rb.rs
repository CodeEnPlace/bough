use super::LanguageDriver;
use crate::mutant::{
    AssignMutationKind, BinaryOpMutationKind, LiteralKind, MutantKind, Span, span_from_node,
};
use tracing::trace;

pub struct RubyDriver;

impl LanguageDriver for RubyDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_ruby::language().into()
    }

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        let result = match node.kind() {
            "binary" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+" => BinaryOpMutationKind::Add,
                    "-" => BinaryOpMutationKind::Sub,
                    "*" => BinaryOpMutationKind::Mul,
                    "/" => BinaryOpMutationKind::Div,
                    "%" => BinaryOpMutationKind::Rem,
                    "==" => BinaryOpMutationKind::Eq,
                    "!=" => BinaryOpMutationKind::Ne,
                    ">" => BinaryOpMutationKind::Gt,
                    ">=" => BinaryOpMutationKind::Gte,
                    "<" => BinaryOpMutationKind::Lt,
                    "<=" => BinaryOpMutationKind::Lte,
                    "&&" => BinaryOpMutationKind::And,
                    "||" => BinaryOpMutationKind::Or,
                    "&" => BinaryOpMutationKind::BitAnd,
                    "|" => BinaryOpMutationKind::BitOr,
                    "^" => BinaryOpMutationKind::BitXor,
                    "<<" => BinaryOpMutationKind::Shl,
                    ">>" => BinaryOpMutationKind::Shr,
                    _ => return None,
                };
                Some((
                    MutantKind::BinaryOp(kind),
                    span_from_node(&op_node),
                    span_from_node(node),
                ))
            }
            "assignment" => {
                let op_node = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == "=")?;
                Some((
                    MutantKind::Assign(AssignMutationKind::NormalAssign),
                    span_from_node(&op_node),
                    span_from_node(node),
                ))
            }
            "operator_assignment" => {
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
            "if" | "while" => {
                let condition = node.child_by_field_name("condition")?;
                Some((
                    MutantKind::Condition,
                    span_from_node(&condition),
                    span_from_node(node),
                ))
            }
            "body_statement" => {
                let span = span_from_node(node);
                Some((MutantKind::StatementBlock, span.clone(), span))
            }
            "integer" | "float" => {
                let span = span_from_node(node);
                Some((MutantKind::Literal(LiteralKind::Number), span.clone(), span))
            }
            "string" => {
                let text = node.utf8_text(file_content).ok()?;
                let kind = if text == "\"\"" || text == "''" {
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
            "array" if node.named_child_count() > 0 => {
                let span = span_from_node(node);
                Some((
                    MutantKind::ArrayDecl(crate::mutant::ArrayDeclKind::Inline),
                    span.clone(),
                    span,
                ))
            }
            "hash" if node.named_child_count() > 0 => {
                let span = span_from_node(node);
                Some((MutantKind::DictDecl, span.clone(), span))
            }
            "unary" => {
                let op_node = node.child_by_field_name("operator")?;
                if op_node.kind() == "!" {
                    Some((
                        MutantKind::UnaryNot,
                        span_from_node(&op_node),
                        span_from_node(node),
                    ))
                } else {
                    None
                }
            }
            "when" if node.child_by_field_name("body").is_some() => {
                let span = span_from_node(node);
                Some((MutantKind::SwitchCase, span.clone(), span))
            }
            _ => None,
        };
        if let Some((ref kind, ref span, _)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "rb: matched node");
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
            MutantKind::BinaryOp(BinaryOpMutationKind::Eq) => vec!["!=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Ne) => vec!["==".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gt) => vec!["<=".into(), ">=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gte) => vec!["<".into(), ">".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lt) => vec![">=".into(), "<=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lte) => vec![">".into(), "<".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::And) => vec!["||".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Or) => vec!["&&".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitAnd) => vec!["|".into(), "^".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitOr) => vec!["&".into(), "^".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::BitXor) => vec!["&".into(), "|".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Shl) => vec![">>".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Shr) => vec!["<<".into()],
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
            MutantKind::Condition => vec!["true".into(), "false".into()],
            MutantKind::StatementBlock => vec!["nil".into()],
            MutantKind::Literal(LiteralKind::Number) => {
                vec![
                    "0".into(),
                    "1".into(),
                    "-1".into(),
                    "Float::INFINITY".into(),
                    "-Float::INFINITY".into(),
                    "Float::NAN".into(),
                ]
            }
            MutantKind::Literal(LiteralKind::String) => vec!["\"\"".into()],
            MutantKind::Literal(LiteralKind::EmptyString) => vec!["\"bough\"".into()],
            MutantKind::Literal(LiteralKind::BoolTrue) => vec!["false".into()],
            MutantKind::Literal(LiteralKind::BoolFalse) => vec!["true".into()],
            MutantKind::ArrayDecl(crate::mutant::ArrayDeclKind::Inline) => vec!["[]".into()],
            MutantKind::DictDecl => vec!["{}".into()],
            MutantKind::UnaryNot => vec!["".into()],
            MutantKind::SwitchCase => vec!["".into()],
            _ => vec![],
        }
    }

    fn is_context_boundary(&self, node: &arborium_tree_sitter::Node<'_>) -> bool {
        matches!(
            node.kind(),
            "method" | "class" | "module" | "singleton_method"
        )
    }
}

#[cfg(test)]
mod tests {
    fn dump_tree(src: &str) {
        let lang: arborium_tree_sitter::Language = arborium_ruby::language().into();
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
        dump_tree("case x\nwhen 1\n  \"one\"\nwhen 2\n  \"two\"\nend\n");
    }
}

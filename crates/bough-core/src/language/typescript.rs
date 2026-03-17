use super::LanguageDriver;
use crate::mutant::{
    ArrayDeclKind, AssignMutationKind, BinaryOpMutationKind, LiteralKind, MutantKind,
    OptionalChainKind, Span, span_from_node,
};
use tracing::trace;

pub(crate) struct TypescriptDriver;

impl LanguageDriver for TypescriptDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_typescript::language().into()
    }

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        let result = match node.kind() {
            "statement_block" => {
                let span = span_from_node(node);
                Some((MutantKind::StatementBlock, span.clone(), span))
            }
            "if_statement" | "while_statement" | "for_statement" => {
                let condition = node.child_by_field_name("condition")?;
                let inner = if condition.kind() == "parenthesized_expression" {
                    condition.child(1).unwrap_or(condition)
                } else {
                    condition
                };
                Some((MutantKind::Condition, span_from_node(&inner), span_from_node(node)))
            }
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
                    "===" => BinaryOpMutationKind::StrictEq,
                    "!=" => BinaryOpMutationKind::Ne,
                    "!==" => BinaryOpMutationKind::StrictNe,
                    ">" => BinaryOpMutationKind::Gt,
                    ">=" => BinaryOpMutationKind::Gte,
                    "<" => BinaryOpMutationKind::Lt,
                    "<=" => BinaryOpMutationKind::Lte,
                    _ => return None,
                };
                Some((MutantKind::BinaryOp(kind), span_from_node(&op_node), span_from_node(node)))
            }
            "augmented_assignment_expression" => {
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
                    "&&=" => AssignMutationKind::AndAssign,
                    "||=" => AssignMutationKind::OrAssign,
                    _ => return None,
                };
                Some((MutantKind::Assign(kind), span_from_node(&op_node), span_from_node(node)))
            }
            "assignment_expression" => {
                let op_node = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == "=")?;
                Some((
                    MutantKind::Assign(AssignMutationKind::NormalAssign),
                    span_from_node(&op_node),
                    span_from_node(node),
                ))
            }
            "array" if node.named_child_count() > 0 => {
                let span = span_from_node(node);
                Some((MutantKind::ArrayDecl(ArrayDeclKind::Inline), span.clone(), span))
            }
            "new_expression" => {
                let callee = node.child_by_field_name("constructor")?;
                if callee.utf8_text(file_content).ok()? == "Array" {
                    let args = node.child_by_field_name("arguments")?;
                    if args.named_child_count() > 0 {
                        let span = span_from_node(node);
                        Some((MutantKind::ArrayDecl(ArrayDeclKind::Instance), span.clone(), span))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "object" if node.named_child_count() > 0 => {
                let span = span_from_node(node);
                Some((MutantKind::DictDecl, span.clone(), span))
            }
            "true" => {
                let span = span_from_node(node);
                Some((MutantKind::Literal(LiteralKind::BoolTrue), span.clone(), span))
            }
            "false" => {
                let span = span_from_node(node);
                Some((MutantKind::Literal(LiteralKind::BoolFalse), span.clone(), span))
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
            "number" => {
                let span = span_from_node(node);
                Some((MutantKind::Literal(LiteralKind::Number), span.clone(), span))
            }
            "switch_case" | "switch_default" => {
                let parent = node.parent()?;
                if parent.kind() != "switch_body" { return None; }
                let switch_stmt = parent.parent()?;
                let span = span_from_node(node);
                Some((MutantKind::SwitchCase, span, span_from_node(&switch_stmt)))
            }
            "member_expression" | "subscript_expression" | "call_expression" => {
                let oc = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == "?." || c.kind() == "optional_chain")?;
                let kind = match node.kind() {
                    "member_expression" => OptionalChainKind::Literal,
                    "subscript_expression" => OptionalChainKind::Indexed,
                    "call_expression" => OptionalChainKind::FnCall,
                    _ => unreachable!(),
                };
                Some((MutantKind::OptionalChain(kind), span_from_node(&oc), span_from_node(node)))
            }
            _ => None,
        };
        if let Some((ref kind, ref span, _)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "ts: matched node");
        }
        result
    }

    fn is_context_boundary(&self, node: &arborium_tree_sitter::Node<'_>) -> bool {
        matches!(
            node.kind(),
            "function_declaration"
                | "method_definition"
                | "class_declaration"
                | "arrow_function"
                | "export_statement"
        )
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
            MutantKind::BinaryOp(BinaryOpMutationKind::Xor) => vec![],
            MutantKind::BinaryOp(BinaryOpMutationKind::Eq) => vec!["!=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::StrictEq) => vec!["!==".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Ne) => vec!["==".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::StrictNe) => vec!["===".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gt) => vec!["<=".into(), ">=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Gte) => vec!["<".into(), ">".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lt) => vec![">=".into(), "<=".into()],
            MutantKind::BinaryOp(BinaryOpMutationKind::Lte) => vec![">".into(), "<".into()],
            MutantKind::StatementBlock => vec!["{}".into()],
            MutantKind::Condition => vec!["true".into(), "false".into()],
            MutantKind::Assign(AssignMutationKind::NormalAssign) => vec!["+=".into(), "-=".into()],
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
            MutantKind::Assign(AssignMutationKind::AndAssign) => vec!["||=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::OrAssign) => vec!["&&=".into(), "=".into()],
            MutantKind::ArrayDecl(ArrayDeclKind::Inline) => vec!["[]".into()],
            MutantKind::ArrayDecl(ArrayDeclKind::Instance) => vec!["new Array()".into()],
            MutantKind::DictDecl => vec!["{}".into()],
            MutantKind::Literal(LiteralKind::BoolTrue) => vec!["false".into()],
            MutantKind::Literal(LiteralKind::BoolFalse) => vec!["true".into()],
            MutantKind::Literal(LiteralKind::String) => vec!["\"\"".into()],
            MutantKind::Literal(LiteralKind::EmptyString) => vec!["\"bough\"".into()],
            MutantKind::Literal(LiteralKind::Number) => vec![
                "0".into(),
                "1".into(),
                "-1".into(),
                "Infinity".into(),
                "-Infinity".into(),
                "NaN".into(),
            ],
            MutantKind::OptionalChain(OptionalChainKind::Literal) => vec![".".into()],
            MutantKind::OptionalChain(OptionalChainKind::Indexed) => vec!["".into()],
            MutantKind::OptionalChain(OptionalChainKind::FnCall) => vec!["".into()],
            MutantKind::SwitchCase => vec!["".into()],
        }
    }
}


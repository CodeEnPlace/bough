use super::LanguageDriver;
use crate::mutant::{
    ArrayDeclKind, AssignMutationKind, BinaryOpMutationKind, LiteralKind, MutantKind,
    OptionalChainKind, Span, span_from_node,
};
use tracing::trace;

pub(crate) struct TypescriptDriver;

impl LanguageDriver for TypescriptDriver {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn check_node(
        &self,
        node: &tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span)> {
        let result = match node.kind() {
            "statement_block" => Some((MutantKind::StatementBlock, span_from_node(node))),
            "if_statement" | "while_statement" | "for_statement" => {
                let condition = node.child_by_field_name("condition")?;
                let inner = if condition.kind() == "parenthesized_expression" {
                    condition.child(1).unwrap_or(condition)
                } else {
                    condition
                };
                Some((MutantKind::Condition, span_from_node(&inner)))
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
                Some((MutantKind::BinaryOp(kind), span_from_node(&op_node)))
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
                Some((MutantKind::Assign(kind), span_from_node(&op_node)))
            }
            "assignment_expression" => {
                let op_node = (0..node.child_count())
                    .filter_map(|i| node.child(i as u32))
                    .find(|c| c.kind() == "=")?;
                Some((
                    MutantKind::Assign(AssignMutationKind::NormalAssign),
                    span_from_node(&op_node),
                ))
            }
            "array" if node.named_child_count() > 0 => Some((
                MutantKind::ArrayDecl(ArrayDeclKind::Inline),
                span_from_node(node),
            )),
            "new_expression" => {
                let callee = node.child_by_field_name("constructor")?;
                if callee.utf8_text(file_content).ok()? == "Array" {
                    let args = node.child_by_field_name("arguments")?;
                    if args.named_child_count() > 0 {
                        Some((
                            MutantKind::ArrayDecl(ArrayDeclKind::Instance),
                            span_from_node(node),
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            "object" if node.named_child_count() > 0 => {
                Some((MutantKind::DictDecl, span_from_node(node)))
            }
            "true" => Some((
                MutantKind::Literal(LiteralKind::BoolTrue),
                span_from_node(node),
            )),
            "false" => Some((
                MutantKind::Literal(LiteralKind::BoolFalse),
                span_from_node(node),
            )),
            "string" => {
                let text = node.utf8_text(file_content).ok()?;
                let kind = if text == "\"\"" || text == "''" {
                    LiteralKind::EmptyString
                } else {
                    LiteralKind::String
                };
                Some((MutantKind::Literal(kind), span_from_node(node)))
            }
            "number" => Some((
                MutantKind::Literal(LiteralKind::Number),
                span_from_node(node),
            )),
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
                Some((MutantKind::OptionalChain(kind), span_from_node(&oc)))
            }
            _ => None,
        };
        if let Some((ref kind, ref span)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "ts: matched node");
        }
        result
    }

    fn is_context_boundary(&self, node: &tree_sitter::Node<'_>) -> bool {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::LanguageId;
    use crate::base::Base;
    use crate::file::Twig;
    use crate::mutant::TwigMutantsIter;
    use crate::mutation::MutationIter;
    use crate::twig::TwigsIterBuilder;
    use std::path::PathBuf;

    fn make_ts_base(content: &str) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.ts"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.ts"),
        )
        .unwrap();
        (dir, base)
    }

    fn all_mutations(src: &str) -> Vec<String> {
        let (_dir, base) = make_ts_base(src);
        let twig = Twig::new(PathBuf::from("src/a.ts")).unwrap();
        TwigMutantsIter::new(LanguageId::Typescript, &base, &twig)
            .unwrap()
            .flat_map(|bm| {
                let mutant = bm.into_mutant();
                MutationIter::new(&mutant)
                    .map(|mutation| mutation.apply_to_complete_src_string(src))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn statement_block() {
        let src = "function f(): number { return 1; }";
        let mutations = all_mutations(src);
        assert!(mutations.contains(&"function f(): number {}".to_string()));
    }

    #[test]
    fn condition() {
        let src = "if (x) { y(); }";
        let mutations = all_mutations(src);
        assert!(mutations.contains(&"if (true) { y(); }".to_string()));
        assert!(mutations.contains(&"if (false) { y(); }".to_string()));
    }

    #[test]
    fn bin_op_add() {
        let src = "const x = 1 + 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 - 2".to_string()));
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
    }

    #[test]
    fn bin_op_sub() {
        let src = "const x = 1 - 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 + 2".to_string()));
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
    }

    #[test]
    fn bin_op_mul() {
        let src = "const x = 1 * 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
        assert!(mutations.contains(&"const x = 1 + 2".to_string()));
    }

    #[test]
    fn bin_op_div() {
        let src = "const x = 1 / 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
        assert!(mutations.contains(&"const x = 1 - 2".to_string()));
    }

    #[test]
    fn bin_op_rem() {
        let src = "const x = 1 % 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
    }

    #[test]
    fn bin_op_bit_and() {
        let src = "const x = 1 & 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 | 2".to_string()));
        assert!(mutations.contains(&"const x = 1 ^ 2".to_string()));
    }

    #[test]
    fn bin_op_bit_or() {
        let src = "const x = 1 | 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 & 2".to_string()));
        assert!(mutations.contains(&"const x = 1 ^ 2".to_string()));
    }

    #[test]
    fn bin_op_bit_xor() {
        let src = "const x = 1 ^ 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 14);
        assert!(mutations.contains(&"const x = 1 & 2".to_string()));
        assert!(mutations.contains(&"const x = 1 | 2".to_string()));
    }

    #[test]
    fn bin_op_shl() {
        let src = "const x = 1 << 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 13);
        assert!(mutations.contains(&"const x = 1 >> 2".to_string()));
    }

    #[test]
    fn bin_op_shr() {
        let src = "const x = 1 >> 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 13);
        assert!(mutations.contains(&"const x = 1 << 2".to_string()));
    }

    #[test]
    fn normal_assign() {
        let src = "x = 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x += 1".to_string()));
        assert!(mutations.contains(&"x -= 1".to_string()));
    }

    #[test]
    fn add_assign() {
        let src = "x += 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x -= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn sub_assign() {
        let src = "x -= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x += 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn mul_assign() {
        let src = "x *= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x /= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn div_assign() {
        let src = "x /= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x *= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn rem_assign() {
        let src = "x %= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x *= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn bit_and_assign() {
        let src = "x &= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x |= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn bit_or_assign() {
        let src = "x |= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x &= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn bit_xor_assign() {
        let src = "x ^= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x &= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn shl_assign() {
        let src = "x <<= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x >>= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn shr_assign() {
        let src = "x >>= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x <<= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn bin_op_and() {
        let src = "const x = a && b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a || b".to_string()));
    }

    #[test]
    fn bin_op_or() {
        let src = "const x = a || b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a && b".to_string()));
    }

    #[test]
    fn bin_op_xor() {
        let src = "const x = a ?? b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 0);
    }

    #[test]
    fn bin_op_eq() {
        let src = "const x = a == b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a != b".to_string()));
    }

    #[test]
    fn bin_op_strict_eq() {
        let src = "const x = a === b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a !== b".to_string()));
    }

    #[test]
    fn bin_op_ne() {
        let src = "const x = a != b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a == b".to_string()));
    }

    #[test]
    fn bin_op_strict_ne() {
        let src = "const x = a !== b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = a === b".to_string()));
    }

    #[test]
    fn bin_op_gt() {
        let src = "const x = a > b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = a <= b".to_string()));
        assert!(mutations.contains(&"const x = a >= b".to_string()));
    }

    #[test]
    fn bin_op_gte() {
        let src = "const x = a >= b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = a < b".to_string()));
        assert!(mutations.contains(&"const x = a > b".to_string()));
    }

    #[test]
    fn bin_op_lt() {
        let src = "const x = a < b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = a >= b".to_string()));
        assert!(mutations.contains(&"const x = a <= b".to_string()));
    }

    #[test]
    fn bin_op_lte() {
        let src = "const x = a <= b";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = a > b".to_string()));
        assert!(mutations.contains(&"const x = a < b".to_string()));
    }

    #[test]
    fn and_assign() {
        let src = "x &&= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x ||= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn or_assign() {
        let src = "x ||= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 8);
        assert!(mutations.contains(&"x &&= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn array_decl_inline() {
        let src = "[1,2,3]";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 19);
        assert!(mutations.contains(&"[]".to_string()));
    }

    #[test]
    fn array_decl_instance() {
        let src = "new Array(1, 2, 3)";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 19);
        assert!(mutations.contains(&"new Array()".to_string()));
    }

    #[test]
    fn literal_bool_true() {
        let src = "true";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"false".to_string()));
    }

    #[test]
    fn literal_bool_false() {
        let src = "false";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"true".to_string()));
    }

    #[test]
    fn dict_decl() {
        let src = "const x = {foo: 1, bar: null}";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 7);
        assert!(mutations.contains(&"const x = {}".to_string()));
    }

    #[test]
    fn optional_chain_literal() {
        let src = "const x = y?.z";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = y.z".to_string()));
    }

    #[test]
    fn optional_chain_indexed() {
        let src = "const x = y?.['z']";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = y['z']".to_string()));
    }

    #[test]
    fn optional_chain_fn_call() {
        let src = "const x = y?.()";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = y()".to_string()));
    }

    #[test]
    fn literal_string() {
        let src = "'foo'";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"\"\"".to_string()));

        let src = "\"foo\"";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"\"\"".to_string()));
    }

    #[test]
    fn literal_string_empty() {
        let src = "''";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"\"bough\"".to_string()));

        let src = "\"\"";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"\"bough\"".to_string()));
    }

    #[test]
    fn literal_number() {
        let src = "123";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 6);
        assert!(mutations.contains(&"0".to_string()));
        assert!(mutations.contains(&"1".to_string()));
        assert!(mutations.contains(&"-1".to_string()));
        assert!(mutations.contains(&"Infinity".to_string()));
        assert!(mutations.contains(&"-Infinity".to_string()));
        assert!(mutations.contains(&"NaN".to_string()));
    }
}

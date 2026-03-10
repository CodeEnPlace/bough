use super::LanguageDriver;
use crate::mutant::{AssignMutationKind, BinaryOpMutationKind, MutantKind, Span, span_from_node};
use tracing::trace;

pub(crate) struct JavascriptDriver;

// bough[impl mutant.twig-iter.find.js.statement]
// bough[impl mutant.twig-iter.find.js.condition.if]
// bough[impl mutant.twig-iter.find.js.condition.while]
// bough[impl mutant.twig-iter.find.js.condition.for]
impl LanguageDriver for JavascriptDriver {
    fn ts_language(&self) -> tree_sitter::Language {
        tree_sitter_javascript::LANGUAGE.into()
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
            // bough[impl mutant.twig-iter.find.js.binary.add]
            // bough[impl mutant.twig-iter.find.js.binary.sub]
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
            _ => None,
        };
        if let Some((ref kind, ref span)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "js: matched node");
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

    // bough[impl mutation.subst.js.add.sub]
    // bough[impl mutation.subst.js.add.mul]
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
            // bough[impl mutation.subst.js.statement]
            MutantKind::StatementBlock => vec!["{}".into()],
            // bough[impl mutation.subst.js.cond.true]
            // bough[impl mutation.subst.js.cond.false]
            MutantKind::Condition => vec!["true".into(), "false".into()],
            MutantKind::Assign(AssignMutationKind::NormalAssign) => {
                vec!["+=".into(), "-=".into()]
            }
            MutantKind::Assign(AssignMutationKind::AddAssign) => vec!["-=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::SubAssign) => vec!["+=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::MulAssign) => vec!["/=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::DivAssign) => vec!["*=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::RemAssign) => vec!["*=".into(), "=".into()],
            MutantKind::Assign(AssignMutationKind::BitAndAssign) => {
                vec!["|=".into(), "=".into()]
            }
            MutantKind::Assign(AssignMutationKind::BitOrAssign) => {
                vec!["&=".into(), "=".into()]
            }
            MutantKind::Assign(AssignMutationKind::BitXorAssign) => {
                vec!["&=".into(), "=".into()]
            }
            MutantKind::Assign(AssignMutationKind::ShlAssign) => {
                vec![">>=".into(), "=".into()]
            }
            MutantKind::Assign(AssignMutationKind::ShrAssign) => {
                vec!["<<=".into(), "=".into()]
            }
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

    fn make_js_base(content: &str) -> (tempfile::TempDir, Base) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.js"), content).unwrap();
        let base = Base::new(
            dir.path().to_path_buf(),
            TwigsIterBuilder::new().with_include_glob("src/**/*.js"),
        )
        .unwrap();
        (dir, base)
    }

    fn all_mutations(src: &str) -> Vec<String> {
        let (_dir, base) = make_js_base(src);
        let twig = Twig::new(PathBuf::from("src/a.js")).unwrap();
        TwigMutantsIter::new(LanguageId::Javascript, &base, &twig)
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
    fn make_statement_block() {
        let src = "function f() { return 1; }";
        let mutations = all_mutations(src);
        assert!(mutations.contains(&"function f() {}".to_string()));
    }

    #[test]
    fn make_condition() {
        let src = "if (x) { y(); }";
        let mutations = all_mutations(src);
        assert!(mutations.contains(&"if (true) { y(); }".to_string()));
        assert!(mutations.contains(&"if (false) { y(); }".to_string()));
    }

    #[test]
    fn make_bin_op_add() {
        let src = "const x = 1 + 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 - 2".to_string()));
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
    }

    #[test]
    fn make_bin_op_sub() {
        let src = "const x = 1 - 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 + 2".to_string()));
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
    }

    #[test]
    fn make_bin_op_mul() {
        let src = "const x = 1 * 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
        assert!(mutations.contains(&"const x = 1 + 2".to_string()));
    }

    #[test]
    fn make_bin_op_div() {
        let src = "const x = 1 / 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
        assert!(mutations.contains(&"const x = 1 - 2".to_string()));
    }

    #[test]
    fn make_bin_op_rem() {
        let src = "const x = 1 % 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 * 2".to_string()));
        assert!(mutations.contains(&"const x = 1 / 2".to_string()));
    }

    #[test]
    fn make_bin_op_bit_and() {
        let src = "const x = 1 & 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 | 2".to_string()));
        assert!(mutations.contains(&"const x = 1 ^ 2".to_string()));
    }

    #[test]
    fn make_bin_op_bit_or() {
        let src = "const x = 1 | 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 & 2".to_string()));
        assert!(mutations.contains(&"const x = 1 ^ 2".to_string()));
    }

    #[test]
    fn make_bin_op_bit_xor() {
        let src = "const x = 1 ^ 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"const x = 1 & 2".to_string()));
        assert!(mutations.contains(&"const x = 1 | 2".to_string()));
    }

    #[test]
    fn make_bin_op_shl() {
        let src = "const x = 1 << 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = 1 >> 2".to_string()));
    }

    #[test]
    fn make_bin_op_shr() {
        let src = "const x = 1 >> 2";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 1);
        assert!(mutations.contains(&"const x = 1 << 2".to_string()));
    }

    #[test]
    fn make_normal_assign() {
        let src = "x = 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x += 1".to_string()));
        assert!(mutations.contains(&"x -= 1".to_string()));
    }

    #[test]
    fn make_add_assign() {
        let src = "x += 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x -= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_sub_assign() {
        let src = "x -= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x += 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_mul_assign() {
        let src = "x *= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x /= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_div_assign() {
        let src = "x /= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x *= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_rem_assign() {
        let src = "x %= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x *= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_bit_and_assign() {
        let src = "x &= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x |= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_bit_or_assign() {
        let src = "x |= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x &= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_bit_xor_assign() {
        let src = "x ^= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x &= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_shl_assign() {
        let src = "x <<= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x >>= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }

    #[test]
    fn make_shr_assign() {
        let src = "x >>= 1";
        let mutations = all_mutations(src);
        assert_eq!(mutations.len(), 2);
        assert!(mutations.contains(&"x <<= 1".to_string()));
        assert!(mutations.contains(&"x = 1".to_string()));
    }
}

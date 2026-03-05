use super::LanguageDriver;
use crate::mutant::{BinaryOpMutationKind, MutantKind, Span, span_from_node};

pub(crate) struct JavascriptDriver;

// core[impl mutant.twig-iter.find.js.statement]
// core[impl mutant.twig-iter.find.js.condition.if]
// core[impl mutant.twig-iter.find.js.condition.while]
// core[impl mutant.twig-iter.find.js.condition.for]
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
            // core[impl mutant.twig-iter.find.js.binary.add]
            // core[impl mutant.twig-iter.find.js.binary.sub]
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
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub) => vec!["+".into(), "*".into()],
            // core[impl mutation.subst.js.statement]
            MutantKind::StatementBlock => vec!["{}".into()],
            // core[impl mutation.subst.js.cond.true]
            // core[impl mutation.subst.js.cond.false]
            MutantKind::Condition => vec!["true".into(), "false".into()],
        }
    }
}

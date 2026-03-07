use super::LanguageDriver;
use crate::mutant::{BinaryOpMutationKind, MutantKind, Span, span_from_node};
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
                Some((MutantKind::Condition, span_from_node(&condition)))
            }
            // bough[impl mutant.twig-iter.find.js.binary.add]
            // bough[impl mutant.twig-iter.find.js.binary.sub]
            "binary_expression" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+" => BinaryOpMutationKind::Add,
                    "-" => BinaryOpMutationKind::Sub,
                    _ => return None,
                };
                Some((MutantKind::BinaryOp(kind), span_from_node(&op_node)))
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
            MutantKind::BinaryOp(BinaryOpMutationKind::Sub) => vec!["+".into(), "*".into()],
            // bough[impl mutation.subst.js.statement]
            MutantKind::StatementBlock => vec!["{}".into()],
            // bough[impl mutation.subst.js.cond.true]
            // bough[impl mutation.subst.js.cond.false]
            MutantKind::Condition => vec!["true".into(), "false".into()],
        }
    }
}

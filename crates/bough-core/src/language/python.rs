use super::LanguageDriver;
use crate::mutant::{BinaryOpMutationKind, MutantKind, Span, span_from_node};
use tracing::trace;

pub(crate) struct PythonDriver;

impl LanguageDriver for PythonDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_python::language().into()
    }

    fn check_node(
        &self,
        node: &arborium_tree_sitter::Node<'_>,
        file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        let result = match node.kind() {
            "binary_operator" => {
                let op_node = node.child_by_field_name("operator")?;
                let op_text = op_node.utf8_text(file_content).ok()?;
                let kind = match op_text {
                    "+" => BinaryOpMutationKind::Add,
                    _ => return None,
                };
                Some((MutantKind::BinaryOp(kind), span_from_node(&op_node), span_from_node(node)))
            }
            _ => None,
        };
        if let Some((ref kind, ref span, _)) = result {
            trace!(node_kind = node.kind(), mutant_kind = ?kind, start_byte = span.start().byte(), "py: matched node");
        }
        result
    }

    fn substitutions(&self, kind: &MutantKind) -> Vec<String> {
        match kind {
            MutantKind::BinaryOp(BinaryOpMutationKind::Add) => vec!["-".into(), "*".into()],
            _ => vec![],
        }
    }

    fn is_context_boundary(&self, _node: &arborium_tree_sitter::Node<'_>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    fn dump_tree(src: &str) {
        let lang: arborium_tree_sitter::Language = arborium_python::language().into();
        let mut parser = arborium_tree_sitter::Parser::new();
        parser.set_language(&lang).unwrap();
        let tree = parser.parse(src.as_bytes(), None).unwrap();
        fn print_node(node: &arborium_tree_sitter::Node, src: &[u8], indent: usize) {
            let text = node.utf8_text(src).unwrap_or("");
            let field = node.parent().and_then(|p| {
                (0..p.child_count())
                    .find(|&i| p.child(i as u32).map(|c| c.id() == node.id()).unwrap_or(false))
                    .and_then(|i| p.field_name_for_child(i as u32))
            });
            let field_str = field.map(|f| format!("{f}: ")).unwrap_or_default();
            eprintln!("{:indent$}{field_str}{} [{}-{}] {text:?}", "", node.kind(), node.start_byte(), node.end_byte(), indent=indent);
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
        dump_tree("x = 1 + 2");
    }
}

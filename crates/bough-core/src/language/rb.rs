use super::LanguageDriver;
use crate::mutant::{MutantKind, Span};

pub(crate) struct RubyDriver;

impl LanguageDriver for RubyDriver {
    fn ts_language(&self) -> arborium_tree_sitter::Language {
        arborium_ruby::language().into()
    }

    fn check_node(
        &self,
        _node: &arborium_tree_sitter::Node<'_>,
        _file_content: &[u8],
    ) -> Option<(MutantKind, Span, Span)> {
        None
    }

    fn substitutions(&self, _kind: &MutantKind) -> Vec<String> {
        vec![]
    }

    fn is_context_boundary(&self, _node: &arborium_tree_sitter::Node<'_>) -> bool {
        false
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
        dump_tree(r#"x + y
x - y
x * y
x / y
x % y
x == y
x != y
x > y
x >= y
x < y
x <= y
x && y
x || y
x & y
x | y
x ^ y
x << y
x >> y
x = 1
x += 1
x -= 1
x *= 1
x /= 1
x %= 1
x &= 1
x |= 1
x ^= 1
x <<= 1
x >>= 1
if x > 0
  puts "yes"
end
while x > 0
  x -= 1
end
def foo
  puts "hello"
end
42
0.5
"hello"
""
true
false
[1, 2, 3]
{a: 1, b: 2}
!x
case x
when 1
  "one"
when 2
  "two"
end
"#);
    }
}

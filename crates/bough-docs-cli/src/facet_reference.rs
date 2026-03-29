use std::collections::HashSet;
use std::fmt::Write;

use facet::Facet;
use facet_core::{Def, Field, Shape, Type, UserType};

pub fn make_facet_reference<'a, T: Facet<'a>>() -> String {
    let mut out = String::new();
    let mut visited = HashSet::new();
    render_shape(T::SHAPE, &mut out, 2, &mut visited);
    out
}

fn render_shape(
    shape: &'static Shape,
    out: &mut String,
    heading_level: usize,
    visited: &mut HashSet<&'static str>,
) {
    if !visited.insert(shape.type_identifier) {
        return;
    }

    let prefix = "#".repeat(heading_level);

    match &shape.ty {
        Type::User(UserType::Struct(st)) => {
            let _ = writeln!(out, "{prefix} `{}`\n", shape.type_identifier);
            render_doc(shape.doc, out);
            if st.fields.is_empty() {
                return;
            }

            let _ = writeln!(out, "| Field | Type | Description |");
            let _ = writeln!(out, "|-------|------|-------------|");
            let mut nested = Vec::new();
            for field in st.fields {
                if field.is_flattened() {
                    let inner = field.shape();
                    if let Type::User(UserType::Struct(inner_st)) = &inner.ty {
                        for f in inner_st.fields {
                            render_field_row(f, out, &mut nested);
                        }
                    }
                } else {
                    render_field_row(field, out, &mut nested);
                }
            }
            let _ = writeln!(out);

            for nested_shape in nested {
                render_shape(nested_shape, out, heading_level + 1, visited);
            }
        }
        Type::User(UserType::Enum(et)) => {
            let _ = writeln!(out, "{prefix} `{}`\n", shape.type_identifier);
            render_doc(shape.doc, out);

            let mut nested = Vec::new();
            for variant in et.variants {
                let _ = write!(out, "- **`{}`**", variant.name);
                let doc = join_doc(variant.doc);
                if !doc.is_empty() {
                    let _ = write!(out, " — {doc}");
                }
                let _ = writeln!(out);

                if !variant.data.fields.is_empty() {
                    let _ = writeln!(out);
                    let _ = writeln!(out, "  | Field | Type | Description |");
                    let _ = writeln!(out, "  |-------|------|-------------|");
                    for field in variant.data.fields {
                        render_field_row(field, out, &mut nested);
                    }
                    let _ = writeln!(out);
                }
            }
            let _ = writeln!(out);

            for nested_shape in nested {
                render_shape(nested_shape, out, heading_level + 1, visited);
            }
        }
        _ => {}
    }
}

fn render_field_row(field: &'static Field, out: &mut String, nested: &mut Vec<&'static Shape>) {
    let field_shape = field.shape();
    let type_name = display_type_name(field_shape);
    let doc = join_doc(field.doc);
    let default_note = if field.has_default() {
        " *(optional)*"
    } else {
        ""
    };
    let _ = writeln!(
        out,
        "| `{}` | `{type_name}` | {doc}{default_note} |",
        field.effective_name()
    );

    collect_nested(field_shape, nested);
}

fn collect_nested(shape: &'static Shape, nested: &mut Vec<&'static Shape>) {
    match shape.def {
        Def::Option(opt) => collect_nested(opt.t, nested),
        Def::List(list) => collect_nested(list.t, nested),
        Def::Map(map) => collect_nested(map.v, nested),
        _ => match &shape.ty {
            Type::User(UserType::Struct(_) | UserType::Enum(_)) => {
                nested.push(shape);
            }
            _ => {}
        },
    }
}

fn display_type_name(shape: &'static Shape) -> String {
    match shape.def {
        Def::Option(opt) => format!("{}?", display_type_name(opt.t)),
        Def::List(list) => format!("{}[]", display_type_name(list.t)),
        Def::Map(map) => format!(
            "Map<{}, {}>",
            display_type_name(map.k),
            display_type_name(map.v)
        ),
        _ => shape.type_identifier.to_string(),
    }
}

fn render_doc(doc: &[&str], out: &mut String) {
    let joined = join_doc(doc);
    if !joined.is_empty() {
        let _ = writeln!(out, "{joined}\n");
    }
}

fn join_doc(doc: &[&str]) -> String {
    doc.iter()
        .map(|line| line.strip_prefix(' ').unwrap_or(line))
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet::Facet;

    /// A simple server configuration.
    #[derive(Facet)]
    struct SimpleConfig {
        /// The hostname to bind to.
        host: String,
        /// The port number.
        #[facet(default = 8080)]
        port: u16,
    }

    #[test]
    fn simple_struct_reference() {
        let md = make_facet_reference::<SimpleConfig>();
        assert_eq!(
            md,
            "\
## `SimpleConfig`

A simple server configuration.

| Field | Type | Description |
|-------|------|-------------|
| `host` | `String` | The hostname to bind to. |
| `port` | `u16` | The port number. *(optional)* |

"
        );
    }

    /// Output format.
    #[derive(Facet)]
    #[repr(u8)]
    enum Format {
        /// Compact output.
        Terse,
        /// Detailed output.
        Verbose,
    }

    #[test]
    fn enum_reference() {
        let md = make_facet_reference::<Format>();
        assert_eq!(
            md,
            "\
## `Format`

Output format.

- **`Terse`** — Compact output.
- **`Verbose`** — Detailed output.

"
        );
    }

    /// Top-level config.
    #[derive(Facet)]
    struct NestedConfig {
        /// Display name.
        name: String,
        /// Timeout settings.
        timeout: TimeoutSettings,
    }

    /// Timeout configuration.
    #[derive(Facet)]
    struct TimeoutSettings {
        /// Absolute limit in seconds.
        #[facet(default)]
        absolute: Option<u64>,
        /// Relative multiplier.
        #[facet(default)]
        relative: Option<f64>,
    }

    #[test]
    fn nested_struct_reference() {
        let md = make_facet_reference::<NestedConfig>();
        assert_eq!(
            md,
            "\
## `NestedConfig`

Top-level config.

| Field | Type | Description |
|-------|------|-------------|
| `name` | `String` | Display name. |
| `timeout` | `TimeoutSettings` | Timeout settings. |

### `TimeoutSettings`

Timeout configuration.

| Field | Type | Description |
|-------|------|-------------|
| `absolute` | `u64?` | Absolute limit in seconds. *(optional)* |
| `relative` | `f64?` | Relative multiplier. *(optional)* |

"
        );
    }

    #[derive(Facet)]
    struct WithOptionalVec {
        /// List of patterns.
        #[facet(default)]
        patterns: Option<Vec<String>>,
    }

    #[test]
    fn optional_vec_type_display() {
        let md = make_facet_reference::<WithOptionalVec>();
        assert!(md.contains("| `patterns` | `String[]?` |"));
    }

    /// Flattened parent.
    #[derive(Facet)]
    struct FlatParent {
        /// The command.
        cmd: String,
        /// Overrides.
        #[facet(flatten, default)]
        overrides: FlatChild,
    }

    /// Overrideable fields.
    #[derive(Facet, Default)]
    struct FlatChild {
        /// Working directory.
        #[facet(default)]
        pwd: Option<String>,
    }

    #[test]
    fn flattened_fields_inlined() {
        let md = make_facet_reference::<FlatParent>();
        assert_eq!(
            md,
            "\
## `FlatParent`

Flattened parent.

| Field | Type | Description |
|-------|------|-------------|
| `cmd` | `String` | The command. |
| `pwd` | `String?` | Working directory. *(optional)* |

"
        );
    }
}

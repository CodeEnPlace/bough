// (op, alternatives) — ordered longest-first to avoid substring matches
const BINARY_OP_TABLE: &[(&str, &[&str])] = &[
    ("===", &["!=="]),
    ("!==", &["==="]),
    ("&&",  &["||"]),
    ("||",  &["&&"]),
    ("<=",  &["<", ">=", ">"]),
    (">=",  &[">", "<=", "<"]),
    ("==",  &["!="]),
    ("!=",  &["=="]),
    ("+",   &["-", "*", "/"]),
    ("-",   &["+", "*", "/"]),
    ("*",   &["+", "-", "/"]),
    ("/",   &["+", "-", "*"]),
    ("<",   &[">", "<=", ">="]),
    (">",   &["<", "<=", ">="]),
];

pub fn binary_op_substitutions(span_text: &str) -> Vec<String> {
    for (op, alts) in BINARY_OP_TABLE {
        if let Some(pos) = span_text.find(op) {
            let lhs = &span_text[..pos];
            let rhs = &span_text[pos + op.len()..];
            return alts.iter().map(|alt| format!("{}{}{}", lhs, alt, rhs)).collect();
        }
    }
    vec![]
}

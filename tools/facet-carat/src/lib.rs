use facet_value::Value;

/// The wildcard sentinel stored in parsed JSON.
const CARAT: &str = "^";

fn is_wildcard(value: &Value) -> bool {
    value.as_string().is_some_and(|s| s.as_str() == CARAT)
}

/// Recursively check whether `actual` matches `expected`, treating any expected
/// string value of `"^"` as a wildcard that matches anything.
fn values_match(actual: &Value, expected: &Value) -> bool {
    if is_wildcard(expected) {
        return true;
    }

    match (actual.as_object(), expected.as_object()) {
        (Some(a), Some(e)) => {
            if a.len() != e.len() {
                return false;
            }
            return e
                .iter()
                .all(|(k, ev)| a.get(k.as_str()).is_some_and(|av| values_match(av, ev)));
        }
        (None, None) => {}
        _ => return false,
    }

    match (actual.as_array(), expected.as_array()) {
        (Some(a), Some(e)) => {
            if a.len() != e.len() {
                return false;
            }
            return a.iter().zip(e.iter()).all(|(av, ev)| values_match(&av, &ev));
        }
        (None, None) => {}
        _ => return false,
    }

    actual == expected
}

/// Replace bare `*` tokens (outside of JSON strings) with `"^"` so the result
/// is valid JSON that our wildcard comparator understands.
fn preprocess_wildcards(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_string = false;
    let mut prev_backslash = false;

    for ch in input.chars() {
        if in_string {
            if ch == '\\' && !prev_backslash {
                prev_backslash = true;
                out.push(ch);
                continue;
            }
            if ch == '"' && !prev_backslash {
                in_string = false;
            }
            prev_backslash = false;
            out.push(ch);
        } else {
            if ch == '"' {
                in_string = true;
                out.push(ch);
            } else if ch == '*' {
                out.push_str("\"^\"");
            } else {
                out.push(ch);
            }
        }
    }
    out
}

fn parse_json(json: &str) -> Value {
    facet_json::from_str::<Value>(json)
        .unwrap_or_else(|e| panic!("failed to parse JSON: {e}\n\n{json}"))
}

fn pretty(value: &Value) -> String {
    facet_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

#[doc(hidden)]
pub fn __assert_eq_carat<'f, T: facet::Facet<'f>>(actual: &T, expected_pattern: &str) {
    let actual_json = facet_json::to_string(actual)
        .unwrap_or_else(|e| panic!("failed to serialize actual value: {e}"));
    let actual = parse_json(&actual_json);
    let expected_preprocessed = preprocess_wildcards(expected_pattern);
    let expected = parse_json(&expected_preprocessed);

    if !values_match(&actual, &expected) {
        panic!(
            "JSON mismatch\n\nactual:\n{}\n\nexpected (pattern):\n{}",
            pretty(&actual),
            expected_pattern,
        );
    }
}

/// Assert that a Facet value matches an expected JSON pattern.
///
/// Use `*` in the pattern as a wildcard that matches any value.
///
/// # Example
///
/// ```
/// use facet::Facet;
/// use facet_carat::assert_eq_carat;
///
/// #[derive(Facet)]
/// struct Item { name: String, count: u32 }
///
/// let item = Item { name: "test".into(), count: 42 };
/// assert_eq_carat!(&item, r#"{"name":"test","count":*}"#);
/// ```
#[macro_export]
macro_rules! assert_eq_carat {
    ($actual:expr, $expected:expr $(,)?) => {
        $crate::__assert_eq_carat($actual, $expected)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet::Facet;

    #[derive(Facet)]
    struct Simple {
        name: String,
        status: String,
    }

    #[derive(Facet)]
    struct WithTimestamp {
        name: String,
        at: String,
    }

    #[derive(Facet)]
    struct Nested {
        outer: Inner,
    }

    #[derive(Facet)]
    struct Inner {
        inner: u32,
        at: String,
    }

    #[derive(Facet)]
    struct Single {
        x: u32,
    }

    #[derive(Facet)]
    struct SingleBool {
        x: bool,
    }

    #[derive(Facet)]
    struct SingleOption {
        x: Option<String>,
    }

    #[derive(Facet)]
    struct SingleNested {
        x: Inner,
    }

    #[derive(Facet)]
    struct SingleVec {
        x: Vec<u32>,
    }

    #[derive(Facet)]
    struct SingleString {
        x: String,
    }

    // --- preprocess_wildcards ---

    #[test]
    fn preprocess_bare_wildcard() {
        assert_eq!(preprocess_wildcards(r#"{"x":*}"#), r#"{"x":"^"}"#);
    }

    #[test]
    fn preprocess_wildcard_in_array() {
        assert_eq!(preprocess_wildcards(r#"[*,*]"#), r#"["^","^"]"#);
    }

    #[test]
    fn preprocess_star_inside_string_untouched() {
        assert_eq!(preprocess_wildcards(r#"{"x":"a*b"}"#), r#"{"x":"a*b"}"#);
    }

    #[test]
    fn preprocess_escaped_quote_inside_string() {
        assert_eq!(
            preprocess_wildcards(r#"{"x":"a\"*b"}"#),
            r#"{"x":"a\"*b"}"#
        );
    }

    #[test]
    fn preprocess_no_wildcards() {
        let input = r#"{"x":42}"#;
        assert_eq!(preprocess_wildcards(input), input);
    }

    // --- assert_eq_carat! ---

    #[test]
    fn exact_match() {
        let actual = Simple {
            name: "test".into(),
            status: "Caught".into(),
        };
        assert_eq_carat!(&actual, r#"{"name":"test","status":"Caught"}"#);
    }

    #[test]
    fn wildcard_string_field() {
        let actual = WithTimestamp {
            name: "test".into(),
            at: "2026-04-03T12:00:00Z".into(),
        };
        assert_eq_carat!(&actual, r#"{"name":"test","at":*}"#);
    }

    #[test]
    fn wildcard_nested() {
        let actual = Nested {
            outer: Inner {
                inner: 42,
                at: "2026-04-03T12:00:00Z".into(),
            },
        };
        assert_eq_carat!(&actual, r#"{"outer":{"inner":42,"at":*}}"#);
    }

    #[test]
    #[should_panic(expected = "JSON mismatch")]
    fn mismatch_detected() {
        let actual = Simple {
            name: "test".into(),
            status: "Missed".into(),
        };
        assert_eq_carat!(&actual, r#"{"name":"test","status":"Caught"}"#);
    }

    #[test]
    fn wildcard_matches_number() {
        let actual = Single { x: 42 };
        assert_eq_carat!(&actual, r#"{"x":*}"#);
    }

    #[test]
    fn wildcard_matches_bool() {
        let actual = SingleBool { x: true };
        assert_eq_carat!(&actual, r#"{"x":*}"#);
    }

    #[test]
    fn wildcard_matches_null() {
        let actual = SingleOption { x: None };
        assert_eq_carat!(&actual, r#"{"x":*}"#);
    }

    #[test]
    fn wildcard_matches_object() {
        let actual = SingleNested {
            x: Inner {
                inner: 1,
                at: "now".into(),
            },
        };
        assert_eq_carat!(&actual, r#"{"x":*}"#);
    }

    #[test]
    fn wildcard_matches_array() {
        let actual = SingleVec { x: vec![1, 2, 3] };
        assert_eq_carat!(&actual, r#"{"x":*}"#);
    }

    #[test]
    fn wildcards_in_array_elements() {
        let actual = vec![
            WithTimestamp {
                name: "a".into(),
                at: "2026-01-01".into(),
            },
            WithTimestamp {
                name: "b".into(),
                at: "2026-02-02".into(),
            },
        ];
        assert_eq_carat!(&actual, r#"[{"name":"a","at":*},{"name":"b","at":*}]"#);
    }

    #[test]
    #[should_panic(expected = "JSON mismatch")]
    fn extra_key_in_actual() {
        let actual = Simple {
            name: "test".into(),
            status: "Caught".into(),
        };
        assert_eq_carat!(&actual, r#"{"name":"test"}"#);
    }

    #[test]
    #[should_panic(expected = "JSON mismatch")]
    fn missing_key_in_actual() {
        let actual = SingleString {
            x: "test".into(),
        };
        assert_eq_carat!(&actual, r#"{"x":"test","status":"Caught"}"#);
    }

    #[test]
    #[should_panic(expected = "JSON mismatch")]
    fn array_length_mismatch() {
        let actual = vec![1u32, 2, 3];
        assert_eq_carat!(&actual, r#"[1,2]"#);
    }

    #[test]
    fn star_inside_string_is_not_wildcard() {
        let actual = SingleString { x: "a*b".into() };
        assert_eq_carat!(&actual, r#"{"x":"a*b"}"#);
    }

    #[test]
    #[should_panic(expected = "JSON mismatch")]
    fn star_inside_string_does_not_match_other() {
        let actual = SingleString {
            x: "hello".into(),
        };
        assert_eq_carat!(&actual, r#"{"x":"a*b"}"#);
    }
}

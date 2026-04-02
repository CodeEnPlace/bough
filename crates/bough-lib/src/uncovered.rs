pub fn uncovered_fn(a: u32, b: u32) -> &'static str {
    if a > b {
        return "foo";
    }

    if a < b {
        return "bar";
    }

    return "qux";
}

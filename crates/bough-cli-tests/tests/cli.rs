use bough_cli_tests::Fixture;

#[test]
fn smoke_no_args() {
    let fixture = Fixture::new().build();
    let result = fixture.run("");
    assert_eq!(result.code, 1);
    assert!(
        result.stderr.contains("Missing required fields"),
        "expected missing fields error, got stderr:\n{}",
        result.stderr
    );
    assert!(
        result.stderr.contains("command"),
        "expected 'command' in missing fields, got stderr:\n{}",
        result.stderr
    );
    assert_eq!(result.stdout, "");
}

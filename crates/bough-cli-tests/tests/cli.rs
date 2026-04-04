use bough_cli_tests::Fixture;

#[test]
fn smoke_no_args() {
    let fixture = Fixture::new().build();
    let result = fixture.run("");
    assert_eq!(result.code, 1);
    let stderr = result.redacted_stderr(&fixture);
    assert!(
        stderr.contains("\
Missing:
  command <Subcommand> (<command>)
  base_root_dir <String> (--config.base-root-dir or $BOUGH__BASE_ROOT_DIR)
  include <Vec<String>> (--config.include or $BOUGH__INCLUDE)
  exclude <Vec<String>> (--config.exclude or $BOUGH__EXCLUDE)
  lang <HashMap<LanguageId, LanguageConfig>> (--config.lang or $BOUGH__LANG)"),
        "expected missing fields summary, got stderr:\n{stderr}",
    );
    assert_eq!(result.stdout, "");
}

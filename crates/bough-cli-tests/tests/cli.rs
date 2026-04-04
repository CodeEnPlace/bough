use bough_cli_tests::Fixture;

#[test]
fn smoke_no_args() {
    let fixture = Fixture::new().build();
    let result = fixture.run("");
    assert_eq!(result.code, 1);
    assert_eq!(result.stdout, "");
    assert_eq!(
        result.redacted_stderr(&fixture),
        "\
Error: Missing required fields:

Sources:
├─ file: <CONFIG_SEARCH_PATHS>
├─ env $BOUGH__*
├─ cli --config.*
└─ defaults

verbose........ 0.......... DEFAULT
format......... terse...... DEFAULT
no_color....... false...... DEFAULT
command........ ........... ⨯ MISSING
workers........ 1.......... DEFAULT
threads........ 1.......... DEFAULT
base_root_dir.. ........... ⨯ MISSING
include........ ........... ⨯ MISSING
exclude........ ........... ⨯ MISSING
lang........... ........... ⨯ MISSING
pwd............ <default>.. DEFAULT
env............ <default>.. DEFAULT
timeout........ <default>.. DEFAULT
test........... <default>.. DEFAULT
init........... <default>.. DEFAULT
reset.......... <default>.. DEFAULT
find
├─ number........... 1.. DEFAULT
├─ number_per_file.. 1.. DEFAULT
└─ factors
···├─ [0].. EncompasingMissedMutationsCount.. DEFAULT
···└─ [1].. TSNodeDepth...................... DEFAULT

Missing:
  command <Subcommand> (<command>)
  base_root_dir <String> (--config.base-root-dir or $BOUGH__BASE_ROOT_DIR)
  include <Vec<String>> (--config.include or $BOUGH__INCLUDE)
  exclude <Vec<String>> (--config.exclude or $BOUGH__EXCLUDE)
  lang <HashMap<LanguageId, LanguageConfig>> (--config.lang or $BOUGH__LANG)

Run with --help for usage information.

"
    );
}

use bough_cli_test::{TestPlan, cmd};

mod basic {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[runner]
pwd = "."
[runner.js]
files.include = ["*.js"]
"#,
            )
            .file("app.js", "export function add(a, b) { return a + b; }")
    }

    #[test]
    fn finds_mutations() {
        let dir = plan().setup();
        cmd!(dir, "bough show mutations", "mutations for Javascript");
    }

    #[test]
    fn verbose_shows_replacements() {
        let dir = plan().setup();
        cmd!(dir, "bough --output-style verbose show mutations", "BinaryOp");
    }
}

mod empty {
    use super::*;

    #[test]
    fn no_source_no_mutations() {
        let dir = TestPlan::new()
            .config(
                r#"
[runner]
pwd = "."
[runner.js]
files.include = ["*.js"]
"#,
            )
            .setup();

        cmd!(dir, "bough show mutations", "found 0 mutations for Javascript");
    }
}

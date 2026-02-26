use bough_cli_test::{TestPlan, cmd};

mod javascript {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[runner]
pwd = "."
[runner.js]
files.include = ["src/*.js"]
files.exclude = ["**/*.test.*"]
"#,
            )
            .file("src/app.js", "export const x = 1;")
            .file("src/app.test.js", "import { x } from './app.js';")
    }

    #[test]
    fn counts_only_non_test_files() {
        let dir = plan().setup();
        cmd!(dir, "bough show src", "found 1 files for Javascript");
    }

    #[test]
    fn verbose_shows_paths() {
        let dir = plan().setup();
        cmd!(dir, "bough --output-style verbose show src", "Javascript (1 files)");
    }
}

mod typescript {
    use super::*;

    fn plan() -> TestPlan {
        TestPlan::new()
            .config(
                r#"
[runner]
pwd = "."
[runner.ts]
files.include = ["src/*.ts"]
"#,
            )
            .file("src/main.ts", "export const y: number = 2;")
    }

    #[test]
    fn finds_ts_files() {
        let dir = plan().setup();
        cmd!(dir, "bough show src", "found 1 files for Typescript");
    }
}

mod no_files {
    use super::*;

    #[test]
    fn zero_files() {
        let dir = TestPlan::new()
            .config(
                r#"
[runner]
pwd = "."
[runner.js]
files.include = ["src/*.js"]
"#,
            )
            .setup();

        cmd!(dir, "bough show src", "found 0 files for Javascript");
    }
}

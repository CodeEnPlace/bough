use bough_cli_test::{TestPlan, cmd, cmd_err};

fn plan() -> TestPlan {
    TestPlan::new()
        .config(
            r#"
[vcs]
kind = "none"

[dirs]
working = "./work"
state = "./state"

[runner]
pwd = "."
test.commands = ["exit 1"]

[runner.js]
files.include = ["src/*.js"]
files.exclude = []
"#,
        )
        .file("src/app.js", "export function add(a, b) { return a + b; }\n")
        .file("work/.keep", "")
        .file("state/.keep", "")
}

#[test]
fn shows_result_after_test() {
    let dir = plan().setup();

    cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
    cmd!(
        dir,
        "bough --output-style verbose workspace list",
        "{!ws_name} {?ws_path}",
    );
    cmd!(
        dir,
        "bough workspace test {?ws_name} 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "caught mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 in workspace {?ws_path}",
    );
    cmd!(
        dir,
        "bough show mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 caught",
    );
}

#[test]
fn verbose_shows_detail() {
    let dir = plan().setup();

    cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
    cmd!(
        dir,
        "bough --output-style verbose workspace list",
        "{!ws_name} {?ws_path}",
    );
    cmd!(
        dir,
        "bough workspace test {?ws_name} 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "caught mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 in workspace {?ws_path}",
    );
    cmd!(
        dir,
        "bough --output-style verbose show mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 caught",
        "file: {!file_path}",
        "kind: BinaryOp(Add)",
        "replacement: -",
    );
}

#[test]
fn not_found_errors() {
    let dir = plan().setup();

    cmd_err!(
        dir,
        "bough show mutation 0000000000000000000000000000000000000000000000000000000000000000",
        "no mutation result found for hash 0000000000000000000000000000000000000000000000000000000000000000",
    );
}

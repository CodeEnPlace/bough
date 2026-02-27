use bough_cli_test::{TestPlan, cmd};

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
fn removes_stale_results() {
    let dir = plan().setup();

    // create a real result
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

    // plant a stale result
    std::fs::write(
        dir.as_ref().join("state/0000000000000000000000000000000000000000000000000000000000000000.json"),
        "{}",
    )
    .unwrap();

    cmd!(dir, "bough clean", "removed 1 stale results, kept 1");
}

#[test]
fn nothing_to_clean() {
    let dir = plan().setup();

    cmd!(dir, "bough clean", "removed 0 stale results, kept 0");
}

#[test]
fn verbose_lists_removed_hashes() {
    let dir = plan().setup();

    std::fs::write(
        dir.as_ref().join("state/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json"),
        "{}",
    )
    .unwrap();

    cmd!(
        dir,
        "bough --output-style verbose clean",
        "removed 1 stale results, kept 0",
        "removed aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
}

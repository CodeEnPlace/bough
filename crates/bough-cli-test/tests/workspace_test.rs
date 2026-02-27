use bough_cli_test::{TestPlan, assert_whole_file, cmd, cmd_err};

fn plan(test_command: &str) -> TestPlan {
    TestPlan::new()
        .config(&format!(
            r#"
[vcs]
kind = "none"

[dirs]
working = "./work"
state = "./state"

[runner]
pwd = "."
test.commands = ["{test_command}"]

[runner.js]
files.include = ["src/*.js"]
files.exclude = []
"#,
        ))
        .file("src/app.js", "export function add(a, b) { return a + b; }\n")
        .file("work/.keep", "")
        .file("state/.keep", "")
}

#[test]
fn caught_when_test_fails() {
    let dir = plan("exit 1").setup();

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
}

#[test]
fn missed_when_test_passes() {
    let dir = plan("exit 0").setup();

    cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
    cmd!(
        dir,
        "bough --output-style verbose workspace list",
        "{!ws_name} {?ws_path}",
    );
    cmd!(
        dir,
        "bough workspace test {?ws_name} 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "missed mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 in workspace {?ws_path}",
    );
}

#[test]
fn persists_caught_result_to_state_dir() {
    let dir = plan("exit 1").setup();

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

    assert_whole_file!(
        dir,
        "state/7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0.json",
        r#"{
  "outcome": "Caught",
  "at": "{!timestamp}",
  "mutation": {
    "mutant": {
      "src": {
        "path": "{!src_path_1}",
        "language": "javascript"
      },
      "span": {
        "start": {
          "src": {
            "path": "{!src_path_2}",
            "language": "javascript"
          },
          "line": 0,
          "char": 37,
          "byte": 37
        },
        "end": {
          "src": {
            "path": "{!src_path_3}",
            "language": "javascript"
          },
          "line": 0,
          "char": 38,
          "byte": 38
        }
      },
      "kind": {
        "BinaryOp": "Add"
      }
    },
    "replacement": "-"
  }
}"#,
    );
    assert_eq!(src_path_1, src_path_2);
    assert_eq!(src_path_2, src_path_3);
}

#[test]
fn persists_missed_result_to_state_dir() {
    let dir = plan("exit 0").setup();

    cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
    cmd!(
        dir,
        "bough --output-style verbose workspace list",
        "{!ws_name} {?ws_path}",
    );
    cmd!(
        dir,
        "bough workspace test {?ws_name} 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0",
        "missed mutation 7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0 in workspace {?ws_path}",
    );

    assert_whole_file!(
        dir,
        "state/7c43fed8aeccd70bfd659b389dd2647fe348e5f9e97d06d94f25192ed371cfb0.json",
        r#"{
  "outcome": "Missed",
  "at": "{!timestamp}",
  "mutation": {
    "mutant": {
      "src": {
        "path": "{!src_path_1}",
        "language": "javascript"
      },
      "span": {
        "start": {
          "src": {
            "path": "{!src_path_2}",
            "language": "javascript"
          },
          "line": 0,
          "char": 37,
          "byte": 37
        },
        "end": {
          "src": {
            "path": "{!src_path_3}",
            "language": "javascript"
          },
          "line": 0,
          "char": 38,
          "byte": 38
        }
      },
      "kind": {
        "BinaryOp": "Add"
      }
    },
    "replacement": "-"
  }
}"#,
    );
    assert_eq!(src_path_1, src_path_2);
    assert_eq!(src_path_2, src_path_3);
}

#[test]
fn unknown_mutation_hash_fails() {
    let dir = plan("exit 0").setup();

    cmd!(dir, "bough workspace make", "created workspace at {!ws_path}");
    cmd!(
        dir,
        "bough --output-style verbose workspace list",
        "{!ws_name} {?ws_path}",
    );
    cmd_err!(
        dir,
        "bough workspace test {?ws_name} 0000000000000000000000000000000000000000000000000000000000000000",
        "no mutation found with hash 0000000000000000000000000000000000000000000000000000000000000000",
    );
}

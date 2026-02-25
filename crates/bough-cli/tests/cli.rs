use assert_cmd::Command;
use predicates::str::contains;
use std::fs;

fn bough(args: &[&str]) -> assert_cmd::assert::Assert {
    let mut cmd = {
        #[allow(deprecated)]
        Command::cargo_bin("bough").unwrap()
    };
    cmd.args(args);
    cmd.assert()
}

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn no_args_shows_help() {
    bough(&[]).failure().stderr(contains("Usage"));
}

#[test]
fn completions_bash() {
    bough(&["completions", "bash"])
        .success()
        .stdout(contains("complete"));
}

#[test]
fn dump_config_with_explicit_path() {
    bough(&["--config-file", &fixture("full.config.toml"), "--output-style", "json", "show", "config"])
        .success()
        .stdout(contains(r#""parallelism":2"#));
}

#[test]
fn dump_config_missing_file_errors() {
    bough(&["--config-file", "nonexistent.toml", "show", "config"])
        .failure()
        .stderr(contains("nonexistent.toml"));
}

#[test]
fn config_override_file() {
    bough(&[
        "--config-file", &fixture("full.config.toml"),
        "--config-override", &fixture("override.config.toml"),
        "--output-style", "json", "show", "config",
    ])
    .success()
    .stdout(contains(r#""parallelism":99"#));
}

#[test]
fn config_set_inline() {
    bough(&[
        "--config-file", &fixture("full.config.toml"),
        "--config", "parallelism = 42",
        "--output-style", "json", "show", "config",
    ])
    .success()
    .stdout(contains(r#""parallelism":42"#));
}

#[test]
fn config_set_after_override_file() {
    bough(&[
        "--config-file", &fixture("full.config.toml"),
        "--config-override", &fixture("override.config.toml"),
        "--config", "parallelism = 7",
        "--output-style", "json", "show", "config",
    ])
    .success()
    .stdout(contains(r#""parallelism":7"#));
}

#[test]
fn config_discovery_from_cwd() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bough.config.toml"), "parallelism = 5").unwrap();
    let mut cmd = {
        #[allow(deprecated)]
        Command::cargo_bin("bough").unwrap()
    };
    cmd.current_dir(dir.path())
        .args(["--output-style", "json", "show", "config"])
        .assert()
        .success()
        .stdout(contains(r#""parallelism":5"#));
}

#[test]
fn config_discovery_dotconfig_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".config")).unwrap();
    fs::write(
        dir.path().join(".config/bough.config.toml"),
        "parallelism = 3",
    )
    .unwrap();
    let mut cmd = {
        #[allow(deprecated)]
        Command::cargo_bin("bough").unwrap()
    };
    cmd.current_dir(dir.path())
        .args(["--output-style", "json", "show", "config"])
        .assert()
        .success()
        .stdout(contains(r#""parallelism":3"#));
}

#[test]
fn no_config_found_errors() {
    let dir = tempfile::tempdir().unwrap();
    let mut cmd = {
        #[allow(deprecated)]
        Command::cargo_bin("bough").unwrap()
    };
    cmd.current_dir(dir.path())
        .args(["show", "config"])
        .assert()
        .failure()
        .stderr(contains("no config file found"));
}

#[test]
fn unknown_runner_errors() {
    bough(&[
        "--config-file", &fixture("full.config.toml"),
        "--config", "active_runner = \"nonexistent\"",
        "show", "config",
    ])
    .failure()
    .stderr(contains("runner 'nonexistent' not found"))
    .stderr(contains("myrunner"));
}

#[test]
fn valid_runner_succeeds() {
    bough(&[
        "--config-file", &fixture("full.config.toml"),
        "--config", "active_runner = \"myrunner\"",
        "--output-style", "json",
        "show", "config",
    ])
    .success()
    .stdout(contains(r#""active_runner":"myrunner""#));
}

#[test]
fn default_test_phase_is_exit_1() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("bough.config.toml"),
        "[myrunner]\npwd = \".\"",
    )
    .unwrap();
    let mut cmd = {
        #[allow(deprecated)]
        Command::cargo_bin("bough").unwrap()
    };
    cmd.current_dir(dir.path())
        .args(["--output-style", "json", "show", "config"])
        .assert()
        .success()
        .stdout(contains(r#""commands":["exit 1"]"#));
}

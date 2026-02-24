use assert_cmd::Command;
use std::fs;
use std::path::PathBuf;

fn bough() -> Command {
    #[allow(deprecated)]
    Command::cargo_bin("bough").unwrap()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

#[test]
fn no_args_shows_help() {
    bough()
        .assert()
        .failure()
        .stderr(predicates::str::contains("Usage"));
}

#[test]
fn completions_bash() {
    bough()
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicates::str::contains("complete"));
}

#[test]
fn dump_config_with_explicit_path() {
    bough()
        .args(["--config", fixture("full.config.toml").to_str().unwrap(), "--output-style", "json", "dump-config"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":2"#));
}

#[test]
fn dump_config_missing_file_errors() {
    bough()
        .args(["--config", "nonexistent.toml", "dump-config"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("nonexistent.toml"));
}

#[test]
fn config_override_file() {
    bough()
        .args([
            "--config", fixture("full.config.toml").to_str().unwrap(),
            "--config-override", fixture("override.config.toml").to_str().unwrap(),
            "--output-style", "json",
            "dump-config",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":99"#));
}

#[test]
fn config_set_inline() {
    bough()
        .args([
            "--config", fixture("full.config.toml").to_str().unwrap(),
            "--config-set", "parallelism = 42",
            "--output-style", "json",
            "dump-config",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":42"#));
}

#[test]
fn config_set_after_override_file() {
    bough()
        .args([
            "--config", fixture("full.config.toml").to_str().unwrap(),
            "--config-override", fixture("override.config.toml").to_str().unwrap(),
            "--config-set", "parallelism = 7",
            "--output-style", "json",
            "dump-config",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":7"#));
}

#[test]
fn config_discovery_from_cwd() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bough.config.toml"), "parallelism = 5").unwrap();
    bough()
        .current_dir(dir.path())
        .args(["--output-style", "json", "dump-config"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":5"#));
}

#[test]
fn config_discovery_dotconfig_dir() {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir_all(dir.path().join(".config")).unwrap();
    fs::write(dir.path().join(".config/bough.config.toml"), "parallelism = 3").unwrap();
    bough()
        .current_dir(dir.path())
        .args(["--output-style", "json", "dump-config"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""parallelism":3"#));
}

#[test]
fn no_config_found_errors() {
    let dir = tempfile::tempdir().unwrap();
    bough()
        .current_dir(dir.path())
        .args(["dump-config"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("no config file found"));
}

#[test]
fn default_test_phase_is_exit_1() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("bough.config.toml"), r#"
        [myrunner]
        pwd = "."
    "#).unwrap();
    bough()
        .current_dir(dir.path())
        .args(["--output-style", "json", "dump-config"])
        .assert()
        .success()
        .stdout(predicates::str::contains(r#""commands":["exit 1"]"#));
}

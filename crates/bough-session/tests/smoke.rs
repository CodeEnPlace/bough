use bough_session::*;
use std::path::PathBuf;

#[test]
fn merge_cli_overrides_config() {
    let config = PartialSession {
        parallelism: Some(4),
        ..Default::default()
    };
    let cli = PartialSession {
        parallelism: Some(8),
        ..Default::default()
    };
    let merged = cli.merge(config);
    assert_eq!(merged.parallelism, Some(8));
}

#[test]
fn resolve_collects_missing() {
    let partial = PartialSession::default();
    let result = partial.resolve(SessionSkipped {
        config_path: PathBuf::from("test"),
    });
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("vcs_kind"));
}

#[test]
fn resolve_succeeds_with_all_fields() {
    let partial = PartialSession {
        vcs_kind: Some(bough_core::config::VcsKind::Jj),
        directories: PartialDirectories {
            working: Some(PathBuf::from("/tmp")),
            state: Some(PathBuf::from("/tmp/state")),
        },
        parallelism: Some(1),
        ordering: Some(bough_core::config::Ordering::Random),
        files: Some("src/**/*.js".to_string()),
        timeout: PartialTimeout {
            absolute: Some(30),
            relative: Some(1.5),
        },
        commands: PartialCommands {
            test: Some("npm test".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };
    let session = partial.resolve(SessionSkipped {
        config_path: PathBuf::from("test.toml"),
    }).unwrap();

    assert_eq!(session.parallelism, 1);
    assert!(session.commands.install.is_none());
    assert_eq!(session.commands.test, "npm test");
    assert!(!session.exec);
    assert!(!session.no_color);
}

#[test]
fn deserialize_from_toml() {
    let toml_str = r#"
vcs_kind = "jj"
parallelism = 2
ordering = "random"
files = "src/**/*.js"

[directories]
working_dir = "/tmp"

[timeout]
absolute = 30
relative = 1.5

[commands]
test = "npm test"
"#;
    let partial: PartialSession = toml::from_str(toml_str).unwrap();
    assert_eq!(partial.parallelism, Some(2));
    assert_eq!(partial.timeout.absolute, Some(30));
    assert_eq!(partial.commands.test, Some("npm test".to_string()));
}

#[test]
fn vec_merge_override() {
    let config = PartialSession {
        ignore_mutants: vec!["a".to_string()],
        ..Default::default()
    };
    let cli = PartialSession {
        ignore_mutants: vec!["b".to_string()],
        ..Default::default()
    };
    let merged = cli.merge(config);
    assert_eq!(merged.ignore_mutants, vec!["b".to_string()]);
}

#[test]
fn vec_merge_fallback() {
    let config = PartialSession {
        ignore_mutants: vec!["a".to_string()],
        ..Default::default()
    };
    let cli = PartialSession::default();
    let merged = cli.merge(config);
    assert_eq!(merged.ignore_mutants, vec!["a".to_string()]);
}

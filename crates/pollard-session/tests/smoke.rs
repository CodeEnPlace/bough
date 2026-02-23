use pollard_session::*;
use std::path::PathBuf;

#[test]
fn merge_cli_overrides_config() {
    let config = PartialSession {
        language: Some(pollard_core::config::LanguageId::Javascript),
        parallelism: Some(4),
        ..Default::default()
    };
    let cli = PartialSession {
        parallelism: Some(8),
        ..Default::default()
    };
    let merged = cli.merge(config);
    assert_eq!(merged.parallelism, Some(8));
    assert_eq!(merged.language, Some(pollard_core::config::LanguageId::Javascript));
}

#[test]
fn resolve_collects_missing() {
    let partial = PartialSession::default();
    let result = partial.resolve(SessionSkipped {
        config_path: PathBuf::from("test"),
    });
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("language"));
    assert!(err.contains("vcs_kind"));
}

#[test]
fn resolve_succeeds_with_all_fields() {
    let partial = PartialSession {
        language: Some(pollard_core::config::LanguageId::Javascript),
        vcs_kind: Some(pollard_core::config::VcsKind::Jj),
        directories: PartialDirectories {
            working: Some(PathBuf::from("/tmp")),
            report: Some(PathBuf::from("/tmp/reports")),
            ..Default::default()
        },
        parallelism: Some(1),
        ordering: Some(pollard_core::config::Ordering::Random),
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
    assert_eq!(session.directories.sub, PathBuf::from("."));
    assert!(session.commands.install.is_none());
    assert_eq!(session.commands.test, "npm test");
    assert!(!session.exec);
    assert!(!session.no_color);
}

#[test]
fn deserialize_from_toml() {
    let toml_str = r#"
language = "js"
vcs_kind = "jj"
parallelism = 2
ordering = "random"
files = "src/**/*.js"

[directories]
working_dir = "/tmp"
report_dir = "/tmp/reports"

[timeout]
absolute = 30
relative = 1.5

[commands]
test = "npm test"
"#;
    let partial: PartialSession = toml::from_str(toml_str).unwrap();
    assert_eq!(partial.language, Some(pollard_core::config::LanguageId::Javascript));
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

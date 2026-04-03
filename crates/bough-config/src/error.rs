use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum Error {
    #[error("config.include must not be empty")]
    #[diagnostic(
        code(bough::config::empty_include),
        help("add at least one include glob pattern")
    )]
    EmptyInclude,

    #[error("at least one language must be configured")]
    #[diagnostic(
        code(bough::config::no_languages),
        help("add a [lang.*] section to your config")
    )]
    NoLanguages,

    #[error("test.cmd is required")]
    #[diagnostic(
        code(bough::config::missing_test_cmd),
        help("add a [test] section with cmd = \"your test command\"")
    )]
    MissingTestCmd,

    #[error("timeout section must specify at least one of 'absolute' or 'relative'{0}")]
    #[diagnostic(
        code(bough::config::empty_timeout),
        help("add absolute = <seconds> and/or relative = <multiplier>")
    )]
    EmptyTimeout(String),

    #[error("{0}")]
    #[diagnostic(code(bough::config::parse))]
    Parse(String),
}

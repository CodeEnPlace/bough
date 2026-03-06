use std::env;

use facet::Facet;
use figue::{self as args, ConfigFormat, ConfigFormatError, Driver, builder};

#[derive(Facet, Debug)]
struct Cli {
    #[facet(args::subcommand)]
    command: Command,

    #[facet(args::config, args::env_prefix = "BOUGH")]
    config: Config,
}

#[derive(Facet, Debug)]
#[repr(u8)]
enum Command {
    Show {
        #[facet(args::subcommand)]
        what: ShowCommand,
    },
    Run,
}

#[derive(Facet, Debug)]
#[repr(u8)]
enum ShowCommand {
    Cli,
    Config,
    File,
}

#[derive(Facet, Debug)]
struct Config {
    #[facet(default = 4)]
    workers: u64,

    #[facet(default = "default")]
    id: String,
}

struct TomlFormat;

impl ConfigFormat for TomlFormat {
    fn extensions(&self) -> &[&str] {
        &["toml"]
    }

    fn parse(&self, contents: &str) -> Result<figue::ConfigValue, ConfigFormatError> {
        facet_toml::from_str(contents).map_err(|e| ConfigFormatError::new(e.to_string()))
    }
}

struct YamlFormat;

impl ConfigFormat for YamlFormat {
    fn extensions(&self) -> &[&str] {
        &["yaml", "yml"]
    }

    fn parse(&self, contents: &str) -> Result<figue::ConfigValue, ConfigFormatError> {
        facet_yaml::from_str(contents).map_err(|e| ConfigFormatError::new(e.to_string()))
    }
}

const CONFIG_NAMES: &[&str] = &[
    "bough.config.toml",
    "bough.config.yaml",
    "bough.config.yml",
    "bough.config.json",
    ".bough.toml",
    ".bough.yaml",
    ".bough.yml",
    ".bough.json",
    ".config/bough.toml",
    ".config/bough.yaml",
    ".config/bough.yml",
    ".config/bough.json",
];

fn find_config_paths() -> Vec<String> {
    let mut paths = Vec::new();
    let mut dir = env::current_dir().ok();
    while let Some(d) = dir {
        for name in CONFIG_NAMES {
            paths.push(d.join(name).to_string_lossy().into_owned());
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    paths
}

fn resolve_config_path() -> Option<String> {
    find_config_paths()
        .into_iter()
        .find(|p| std::path::Path::new(p).is_file())
}

fn main() {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .env(|env| env)
        .file(|f| {
            f.default_paths(find_config_paths())
                .format(TomlFormat)
                .format(YamlFormat)
        })
        .build();

    let outcome = Driver::new(config).run();
    let output = match outcome.into_result() {
        Ok(output) => output,
        Err(figue::DriverError::Help { text }) => {
            println!("{text}");
            std::process::exit(0);
        }
        Err(figue::DriverError::Failed { report }) => {
            eprintln!("{}", report.render_pretty());
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("{e:?}");
            std::process::exit(1);
        }
    };
    output.print_warnings();

    let config_path = resolve_config_path();
    let (cli, _report) = output.into_parts();

    match cli.command {
        Command::Show { ref what } => match what {
            ShowCommand::Cli => {
                if let Some(ref path) = config_path {
                    println!("config file: {path}");
                }
                println!("{cli:#?}");
            }
            ShowCommand::Config => {
                if let Some(ref path) = config_path {
                    println!("config file: {path}");
                }
                println!("workers: {}", cli.config.workers);
                println!("id: {}", cli.config.id);
            }
            ShowCommand::File => {
                match config_path {
                    Some(ref path) => println!("{path}"),
                    None => println!("no config file found"),
                }
            }
        },
        Command::Run => {
            println!("run: not yet implemented");
            println!("workers: {}", cli.config.workers);
            println!("id: {}", cli.config.id);
        }
    }
}

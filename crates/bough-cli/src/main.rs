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

fn main() {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .env(|env| env)
        .file(|f| f.default_paths(["config.toml"]).format(TomlFormat))
        .build();

    let cli: Cli = Driver::new(config).run().unwrap();

    match cli.command {
        Command::Show { ref what } => match what {
            ShowCommand::Cli => println!("{cli:#?}"),
            ShowCommand::Config => {
                println!("workers: {}", cli.config.workers);
                println!("id: {}", cli.config.id);
            }
            ShowCommand::File => {
                println!("show file: not yet implemented");
            }
        },
        Command::Run => {
            println!("run: not yet implemented");
            println!("workers: {}", cli.config.workers);
            println!("id: {}", cli.config.id);
        }
    }
}

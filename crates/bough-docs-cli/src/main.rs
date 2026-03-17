mod build;
mod facet_reference;
mod serve;

use facet::Facet;
use figue::{self as args, Driver, builder};

#[derive(Facet, Debug)]
struct Cli {
    #[facet(args::named, default = 8787)]
    port: u16,

    #[facet(args::subcommand)]
    command: Command,
}

#[derive(Facet, Debug)]
#[repr(u8)]
enum Command {
    Build,
    Serve,
}

fn main() {
    let config = builder::<Cli>()
        .expect("schema should be valid")
        .cli(|cli| cli)
        .build();

    let outcome = Driver::new(config).run();
    let cli: Cli = match outcome.into_result() {
        Ok(output) => output.value,
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

    match cli.command {
        Command::Build => build::build(),
        Command::Serve => {
            serve::serve(cli.port);
        }
    }
}

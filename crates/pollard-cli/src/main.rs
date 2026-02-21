use clap::Parser;
use log::LevelFilter;

#[derive(Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long, help = "Suppress all log output")]
    quiet: bool,

    #[arg(long, default_value = "test", help = "Run until this step (inclusive)")]
    run_until: Steps,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialOrd, Ord, PartialEq, Eq)]
enum Steps {
    Clone,
    Setup,
    Find,
    Mutate,
    Build,
    Test,
}

fn log_level(cli: &Cli) -> LevelFilter {
    if cli.quiet {
        return LevelFilter::Off;
    }
    match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    }
}

fn main() {
    let cli = Cli::parse();

    env_logger::Builder::new()
        .filter_level(log_level(&cli))
        .parse_default_env()
        .init();

    log::info!("pollard starting");

    log::info!("step: clone");
    if cli.run_until == Steps::Clone { return; }

    log::info!("step: setup");
    if cli.run_until == Steps::Setup { return; }

    log::info!("step: find");
    if cli.run_until == Steps::Find { return; }

    log::info!("step: mutate");
    if cli.run_until == Steps::Mutate { return; }

    log::info!("step: build");
    if cli.run_until == Steps::Build { return; }

    log::info!("step: test");
}

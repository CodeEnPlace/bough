use clap::Parser;
use log::LevelFilter;

#[derive(Parser)]
#[command(name = "pollard", about = "Cross-language mutation testing")]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, help = "Increase log verbosity (-v, -vv, -vvv)")]
    verbose: u8,

    #[arg(short, long, help = "Suppress all log output")]
    quiet: bool,
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
    log::debug!("debug logging enabled");
    log::trace!("trace logging enabled");
}

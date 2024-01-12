use fstored::{
    conf::{self, Config},
    store, ObjectStore,
};

use clap::{Parser, Subcommand};
use std::{path::PathBuf, process::ExitCode};

const COMPILE_CONFIG: Option<&str> = option_env!("FSTORED_DEFAULT_CONFIG");
const DEFAULT_CONFIG: &str = "/etc/fstore/fstore.yml";

#[derive(Parser)]
#[command(
    version,
    about = "Simple object storage server",
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Server config file in YAML format",
        default_value = COMPILE_CONFIG.unwrap_or(DEFAULT_CONFIG),
        global = true
    )]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Retrieve basic information about the server")]
    Status,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    let config = match conf::read(&args.config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    match run(&args, &config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

#[tokio::main]
async fn run(
    args: &Cli,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let store = store::start(config).await?;

    match args.command {
        Command::Status => status(store).await,
    }
}

async fn status(store: ObjectStore) -> Result<(), Box<dyn std::error::Error>> {
    let totals = store.get_totals().await?;

    println!(
        "Buckets: {}\nObjects: {}\nSpace used: {}",
        totals.buckets, totals.objects, totals.space_used
    );

    Ok(())
}

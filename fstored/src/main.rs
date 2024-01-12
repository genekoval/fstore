use fstored::{
    conf::{self, Config},
    store,
};

use clap::Parser;
use std::{path::PathBuf, process::ExitCode};

const COMPILE_CONFIG: Option<&str> = option_env!("FSTORED_DEFAULT_CONFIG");
const DEFAULT_CONFIG: &str = "/etc/fstore/fstore.yml";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
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
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = match conf::read(&cli.config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = run(&config) {
        eprintln!("{err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

#[tokio::main]
async fn run(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let object_store = store::start(config).await?;

    let totals = object_store.get_totals().await?;
    println!(
        "Buckets: {}\nObjects: {}\nSpace used: {}",
        totals.buckets, totals.objects, totals.space_used
    );

    Ok(())
}

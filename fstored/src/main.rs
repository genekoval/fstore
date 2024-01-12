use fstored::conf;

use clap::Parser;
use std::{env, path::PathBuf, process::ExitCode};

const COMPILE_CONFIG: Option<&str> = option_env!("FSTORED_DEFAULT_CONFIG");
const DEFAULT_CONFIG: &str = "/etc/fstore/fstore.yml";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = match cli.config {
        Some(ref config) => config.clone(),
        None => match env::var_os("FSTORED_CONFIG") {
            Some(config) => PathBuf::from(&config),
            None => match COMPILE_CONFIG {
                Some(config) => PathBuf::from(config),
                None => PathBuf::from(DEFAULT_CONFIG),
            },
        },
    };

    let config = match conf::read(&config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    dbg!(config);

    ExitCode::SUCCESS
}

use fstored::conf;

use clap::Parser;
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

const DEFAULT_CONFIG: &str = "/etc/fstore/fstore.yml";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let config = match cli.config.as_deref() {
        Some(config) => config,
        None => Path::new(DEFAULT_CONFIG),
    };

    let config = match conf::read(config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    dbg!(config);

    ExitCode::SUCCESS
}

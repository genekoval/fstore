use fstored::{
    conf::{self, Config},
    server, store, ObjectStore,
};

use clap::{Parser, Subcommand};
use fstore_core::Version;
use shadow_rs::shadow;
use std::{path::PathBuf, process::ExitCode};

shadow!(build);

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const DEFAULT_CONFIG: &str = match option_env!("FSTORED_DEFAULT_CONFIG") {
    Some(config) => config,
    None => "/etc/fstore/fstore.yml",
};

#[derive(Parser)]
#[command(
    version,
    long_version = build::CLAP_LONG_VERSION,
    about = "Simple object storage server",
    arg_required_else_help = true
)]
pub struct Cli {
    #[arg(
        short,
        long,
        value_name = "FILE",
        help = "Server config file in YAML format",
        env = "FSTORE_CONFIG",
        default_value = DEFAULT_CONFIG,
        global = true
    )]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Start the web server")]
    Serve,
    #[command(
        about = "Retrieve basic information about the object repository"
    )]
    Status,
}

fn main() -> ExitCode {
    let args = Cli::parse();

    let mut config = match conf::read(&args.config) {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    if let timber::Sink::Syslog(ref mut syslog) = config.log.sink {
        syslog.identifier = String::from("fstore");
        syslog.logopt = timber::syslog::LogOption::Pid;
    }

    if let Err(err) = timber::new()
        .max_level(config.log.level)
        .sink(config.log.sink.clone())
        .init()
    {
        eprintln!("Failed to initialize logger: {err}");
        return ExitCode::FAILURE;
    }

    match run(&args, &config) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn version() -> Version {
    Version {
        number: build::PKG_VERSION,
        branch: build::BRANCH,
        build_time: build::BUILD_TIME,
        build_os: build::BUILD_OS,
        build_type: build::BUILD_RUST_CHANNEL,
        commit_hash: build::COMMIT_HASH,
        commit_date: build::COMMIT_DATE,
        rust_version: build::RUST_VERSION,
        rust_channel: build::RUST_CHANNEL,
    }
}

#[tokio::main]
async fn run(args: &Cli, config: &Config) -> Result<()> {
    let store = store::start(version(), config).await?;

    match args.command {
        Command::Serve => server::serve(&config.http, store.clone()).await,
        Command::Status => status(&store).await,
    }?;

    store.shutdown().await;
    Ok(())
}

async fn status(store: &ObjectStore) -> Result<()> {
    let totals = store.get_totals().await?;

    println!(
        "Buckets: {}\nObjects: {}\nSpace used: {}",
        totals.buckets, totals.objects, totals.space_used
    );

    Ok(())
}

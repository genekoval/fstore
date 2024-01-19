use fstored::{
    conf::{self, Config},
    server, store, ObjectStore,
};

use clap::{Parser, Subcommand};
use fstore_core::Version;
use log::error;
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
    Serve {
        #[arg(short, long, help = "Run the server as a daemon process")]
        daemon: bool,

        #[arg(short, long, help = "Path to the pidfile", requires = "daemon")]
        pidfile: Option<PathBuf>,
    },

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

    let mut parent = dmon::Parent::default();

    if let Command::Serve { daemon, pidfile } = &args.command {
        if *daemon {
            config.log.sink = timber::Sink::Syslog(timber::syslog::Config {
                identifier: String::from("fstore"),
                logopt: timber::syslog::LogOption::Pid,
                facility: timber::syslog::Facility::Daemon,
            });

            parent = dmon::options()
                .chdir(Some(&config.home))
                .permissions(config.user.as_deref())
                .pidfile(pidfile.as_deref())
                .daemonize();
        }
    } else if let timber::Sink::Syslog(ref mut syslog) = config.log.sink {
        syslog.identifier = String::from("fstore");
        syslog.logopt = timber::syslog::LogOption::Pid;
    }

    match || -> Result<()> {
        timber::new()
            .max_level(config.log.level)
            .sink(config.log.sink.clone())
            .init()
            .map_err(|err| format!("Failed to initialize logger: {err}"))?;

        run(&args, &config, &mut parent)
    }() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            error!("{err}");

            if parent.is_waiting() {
                if let Err(write_error) = parent.write(&format!("{err}")) {
                    error!("Failed to write to parent process: {write_error}");
                }
            }

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
async fn run(
    args: &Cli,
    config: &Config,
    parent: &mut dmon::Parent,
) -> Result<()> {
    let store = store::start(version(), config).await?;

    match args.command {
        Command::Serve { .. } => {
            server::serve(&config.http, store.clone(), parent).await
        }
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

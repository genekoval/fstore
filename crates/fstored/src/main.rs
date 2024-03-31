mod progress;

use progress::ProgressBarTask;

use fstored::{
    conf::{self, Config},
    server, store, ObjectStore, Result,
};

use clap::{Parser, Subcommand};
use fstore_core::Version;
use log::error;
use shadow_rs::shadow;
use std::{future::Future, path::PathBuf, process::ExitCode, sync::Arc};

shadow!(build);

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
        env = "FSTORED_CONFIG",
        default_value = DEFAULT_CONFIG,
        global = true
    )]
    /// Server config file in YAML format
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a backup of the database and object files
    Archive {
        /// Directory to sync data to
        ///
        /// If omitted, the config file's 'archive' setting is used
        directory: Option<PathBuf>,

        #[arg(short, long)]
        /// Do not show progress
        quiet: bool,
    },

    /// Check integrity of objects
    Check {
        #[arg(short, long)]
        /// Do not show progress
        quiet: bool,
    },

    /// Initialize the database
    Init {
        /// Delete existing data if necessary
        overwrite: bool,
    },

    /// Update schemas to match the current program version
    Migrate,

    /// Restore database data and object files from a backup
    Restore {
        /// Directory to restore data from
        ///
        /// If omitted, the config file's 'archive' setting is used
        directory: Option<PathBuf>,

        /// User to connect as
        ///
        /// Restoring may require superuser privileges
        user: Option<String>,
    },

    /// Start the web server
    Serve {
        #[arg(short, long)]
        /// Run the server as a daemon process
        daemon: bool,

        #[arg(short, long, requires = "daemon")]
        /// Path to the pidfile
        pidfile: Option<PathBuf>,
    },
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

    let run = || {
        timber::new()
            .max_level(config.log.level)
            .sink(config.log.sink.clone())
            .init()
            .map_err(|err| format!("Failed to initialize logger: {err}"))?;

        run_async(&args, config, &mut parent)
    };

    if let Err(err) = run() {
        error!("{err}");

        if parent.is_waiting() {
            if let Err(write_error) = parent.write(&format!("{err}")) {
                error!("Failed to write to parent process: {write_error}");
            }
        }

        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
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

async fn store<F, Fut>(config: &Config, f: F) -> Result
where
    F: FnOnce(Arc<ObjectStore>) -> Fut,
    Fut: Future<Output = Result>,
{
    store::start(version(), config, f).await
}

#[tokio::main]
async fn run_async(
    args: &Cli,
    mut config: Config,
    parent: &mut dmon::Parent,
) -> Result {
    match &args.command {
        Command::Archive { directory, quiet } => {
            if let Some(archive) = directory {
                config.archive = Some(archive.clone());
            }

            store(&config, |store| async move {
                let (progress, handle) = store.archive().await?;

                let bar = if *quiet {
                    None
                } else {
                    let total = progress.total();
                    let title = format!(
                        "Syncing {} object{} with archive",
                        total,
                        match total {
                            1 => "",
                            _ => "s",
                        }
                    );

                    Some(ProgressBarTask::new(title, progress.clone()))
                };

                let result = handle.await;

                if let Some(bar) = bar {
                    bar.cancel().await;
                }

                result??;

                let completed = progress.completed();
                let errors = progress.errors();

                println!(
                    "Synced {} object{} with archive{}",
                    completed,
                    match completed {
                        1 => "",
                        _ => "s",
                    },
                    match errors {
                        0 => "".into(),
                        _ => format!(
                            " ({} error{})",
                            errors,
                            match errors {
                                1 => "",
                                _ => "s",
                            }
                        ),
                    }
                );

                Ok(())
            })
            .await
        }
        Command::Check { quiet } => {
            store(&config, |store| async move {
                let (progress, handle) = store.check().await?;

                let bar = if *quiet {
                    None
                } else {
                    let total = progress.total();
                    let title = format!(
                        "Checking {} object{}...",
                        total,
                        match total {
                            1 => "",
                            _ => "s",
                        }
                    );

                    Some(ProgressBarTask::new(title, progress.clone()))
                };

                let result = handle.await;

                if let Some(bar) = bar {
                    bar.cancel().await;
                }

                result??;

                let completed = progress.completed();
                let errors = progress.errors();

                println!(
                    "Checked {} object{} in {}s: {}",
                    completed,
                    match completed {
                        1 => "",
                        _ => "s",
                    },
                    progress.elapsed().num_seconds(),
                    match errors {
                        0 => "all valid".into(),
                        _ => format!(
                            "{} error{}",
                            errors,
                            match errors {
                                1 => "",
                                _ => "s",
                            }
                        ),
                    }
                );

                Ok(())
            })
            .await
        }
        Command::Init { overwrite } => {
            store(&config, |store| async move {
                if *overwrite {
                    store.reset().await?;
                } else {
                    store.init().await?;
                }
                Ok(())
            })
            .await
        }
        Command::Migrate => {
            store(&config, |store| async move {
                store.migrate().await?;
                Ok(())
            })
            .await
        }
        Command::Restore { directory, user } => {
            if let Some(user) = user {
                config
                    .database
                    .connection
                    .params_mut()
                    .insert("user".into(), user.clone());
            }

            let archive = match directory.as_ref().or(config.archive.as_ref()) {
                Some(path) => Ok(path),
                None => Err("no archive location specified"),
            }?;

            store(&config, |store| async move {
                store.restore(archive).await?;
                Ok(())
            })
            .await
        }
        Command::Serve { .. } => {
            store(&config, |store| async {
                server::serve(&config.http, store, parent).await
            })
            .await
        }
    }
}

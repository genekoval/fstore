mod client;
mod conf;
mod print;

use client::{Client, Result};
use conf::Config;
use print::Output;

use clap::{Args, Parser, Subcommand};
use fstore::Uuid;
use std::{path::PathBuf, process::ExitCode, result};

#[derive(Debug, Parser)]
#[command(version, arg_required_else_help = true)]
/// Command-line client for fstore servers
pub struct Cli {
    #[arg(
        short,
        long,
        value_name = "FILE",
        env = "FSTORE_CONFIG",
        global = true
    )]
    /// Config file in TOML format
    config: Option<PathBuf>,

    #[arg(short = 'H', long, env = "FSTORE_HUMAN_READABLE", global = true)]
    /// Print data in a tabulated format
    human_readable: bool,

    #[arg(short, long, env = "FSTORE_JSON", global = true)]
    /// Print data in JSON format
    json: bool,

    #[arg(
        long,
        value_name = "NAME",
        env = "FSTORE_SERVER",
        global = true,
        default_value = "default"
    )]
    /// Name of the server to use
    ///
    /// Server aliases are defined in the config file
    server: String,

    #[command(subcommand)]
    command: Command,
}

impl Cli {
    fn config(&self) -> result::Result<Config, String> {
        Config::read(self.config.clone())
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Get detailed information about the server
    About,

    /// Add an object to a bucket
    Add {
        /// Bucket ID
        bucket: Uuid,

        /// File to upload (STDIN if missing)
        file: Option<PathBuf>,
    },

    Bucket(BucketArgs),

    /// List all buckets
    Buckets,

    /// List object errors
    Errors,

    /// Stream an object's contents
    Get {
        /// Bucket UUID
        bucket: Uuid,

        /// Object UUID
        object: Uuid,

        /// File to stream data to (STDOUT if missing)
        file: Option<PathBuf>,
    },

    /// Delete objects not referenced by a bucket
    Prune {
        /// Print the objects that were deleted
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove objects
    Rm {
        /// Bucket UUID
        bucket: Uuid,

        /// UUIDs of objects to remove
        objects: Vec<Uuid>,
    },

    /// Display object or object repo status
    Stat {
        /// Bucket UUID
        bucket: Option<Uuid>,

        /// Object UUIDs
        object: Option<Vec<Uuid>>,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true, flatten_help = true)]
/// Get information about a bucket
struct BucketArgs {
    #[command(subcommand)]
    command: Option<Bucket>,

    #[command(flatten)]
    get: Option<BucketGetArg>,
}

impl BucketArgs {
    fn command(self) -> Bucket {
        self.command
            .unwrap_or_else(|| Bucket::Get(self.get.unwrap()))
    }
}

#[derive(Debug, Args)]
struct BucketGetArg {
    /// Name of the bucket to retrieve information about
    name: String,
}

#[derive(Debug, Subcommand)]
enum Bucket {
    /// Add a new bucket
    Add {
        /// New bucket's desired name
        name: String,
    },

    /// Create a new bucket containing another bucket's objects
    Clone {
        /// ID of the bucket to clone
        original: Uuid,

        /// The new bucket's name
        name: String,
    },

    /// Retrieve information about a bucket
    Get(BucketGetArg),

    /// Remove a bucket
    Rm {
        /// Bucket UUID
        id: Uuid,
    },

    /// Rename a bucket
    Rename {
        /// Bucket UUID
        id: Uuid,

        /// Bucket's new name
        name: String,
    },
}

fn main() -> ExitCode {
    let args = Cli::parse();
    let config = match args.config() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    if config.servers.is_empty() {
        eprintln!("no servers defined");
        return ExitCode::FAILURE;
    }

    let server = match config.servers.get(&args.server) {
        Some(server) => server,
        None => {
            eprintln!("server alias '{}' not defined", args.server);
            return ExitCode::FAILURE;
        }
    };

    let client = Client::new(
        server,
        Output {
            human_readable: args.human_readable,
            json: args.json,
        },
    );

    match run(args.command, client) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command, client: Client) -> Result {
    let body = async move { run_command(command, client).await };

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("failed to build runtime: {err}"))?
        .block_on(body)
}

async fn run_command(command: Command, client: Client) -> Result {
    match command {
        Command::About => client.about().await,
        Command::Add { bucket, file } => match file {
            Some(file) => client.upload_file(bucket, file).await,
            None => client.stream_stdin(bucket).await,
        },
        Command::Bucket(args) => match args.command() {
            Bucket::Add { name } => client.add_bucket(name).await,
            Bucket::Clone { original, name } => {
                client.clone_bucket(original, name).await
            }
            Bucket::Get(BucketGetArg { name }) => client.get_bucket(name).await,
            Bucket::Rm { id } => client.remove_bucket(id).await,
            Bucket::Rename { id, name } => {
                client.rename_bucket(&id, &name).await
            }
        },
        Command::Buckets => client.get_buckets().await,
        Command::Errors => client.get_object_errors().await,
        Command::Get {
            bucket,
            object,
            file,
        } => client.get_object(bucket, object, file).await,
        Command::Prune { verbose } => client.prune(verbose).await,
        Command::Rm { bucket, objects } => {
            client.remove_objects(bucket, objects).await
        }
        Command::Stat { bucket, object } => match (bucket, object) {
            (Some(bucket), Some(object)) => {
                client.get_objects(bucket, &object).await
            }
            (Some(bucket), None) => client.get_all_objects(bucket).await,
            _ => client.status().await,
        },
    }
}

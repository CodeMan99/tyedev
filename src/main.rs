use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};
#[cfg(feature = "completions")]
use ::{
    clap::CommandFactory,
    clap_complete::{generate, shells::Shell},
};

mod init;
mod inspect;
mod list;
mod oci_ref;
mod registry;
mod search;

/// Easily manage devcontainer configuration files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// Pull the index of features & templates
    #[arg(short, long)]
    pull_index: bool,

    #[command(flatten)]
    verbose: Verbosity<WarnLevel>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Generate shell auto-complete configuration.
    #[cfg(feature = "completions")]
    Completions { shell: Shell },
    /// Create new devcontainer.
    Init(init::InitArgs),
    /// Display details of a specific feature, template, or collection.
    Inspect(inspect::InspectArgs),
    /// Overview of collections.
    List(list::ListArgs),
    /// Text search the `id`, `keywords`, and `description` fields of templates or features.
    Search(search::SearchArgs),
}

fn data_directory<P: AsRef<Path>>(namespace: P) -> io::Result<PathBuf> {
    log::debug!("data_directory");
    if let Some(path) = dirs::data_dir() {
        Ok(path.join(namespace))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Unable to determine a valid data directory",
        ))
    }
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .format_timestamp_millis()
        .init();

    const BIN_NAME: &str = env!("CARGO_BIN_NAME");

    #[cfg(feature = "completions")]
    if let Some(Commands::Completions { shell }) = args.command {
        generate(shell, &mut Args::command_for_update(), BIN_NAME, &mut io::stdout());
        return Ok(());
    }

    let data_dir = data_directory(BIN_NAME)?;
    let index_file = data_dir.join("devcontainer-index.json");

    if args.pull_index {
        if !data_dir.exists() {
            log::debug!("main: Creating data directory");
            fs::create_dir_all(&data_dir)?;
        }

        registry::pull_devcontainer_index(&index_file).await?;
        log::info!("Saved to {}", index_file.display());
    }

    if let Some(command) = args.command {
        if !index_file.exists() {
            // suggested user action
            log::error!("Missing devcontainer-index.json.\n\n\tRun `{BIN_NAME} --pull-index`.\n");
        }

        let index = registry::read_devcontainer_index(index_file)?;

        match command {
            #[cfg(feature = "completions")]
            Commands::Completions { .. } => unreachable!(),
            Commands::Init(args) => init::init(&index, args).await?,
            Commands::Inspect(args) => inspect::inspect(&index, args).await?,
            Commands::List(args) => list::list(&index, args),
            Commands::Search(args) => search::search(&index, args)?,
        };
    }

    Ok(())
}

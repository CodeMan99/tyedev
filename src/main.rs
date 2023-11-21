use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_verbosity_flag::{Verbosity, WarnLevel};

mod init;
mod inspect;
mod list;
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
    /// Create new devcontainer.
    Init (init::InitArgs),
    /// Display details of a specific feature, template, or collection.
    Inspect (inspect::InspectArgs),
    /// Overview of collections.
    List (list::ListArgs),
    /// Text search the `id`, `keywords`, and `description` fields of templates or features.
    Search (search::SearchArgs),
}

fn program_name() -> io::Result<String> {
    log::debug!("program_name");
    let exe = env::current_exe()?;
    exe
    .file_name()
    .and_then(OsStr::to_str)
    .map(String::from)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Executable not a file path"))
}

fn data_directory<P: AsRef<Path>>(namespace: P) -> io::Result<PathBuf> {
    log::debug!("data_directory");
    if let Some(path) = dirs::data_dir() {
        Ok(path.join(namespace))
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to determine a valid data directory"))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    env_logger::Builder::new()
    .filter_level(args.verbose.log_level_filter())
    .format_timestamp_millis()
    .init();

    let prog_name = program_name()?;
    let data_dir = data_directory(&prog_name)?;
    let index_file = data_dir.join("devcontainer-index.json");

    if args.pull_index {
        if !data_dir.exists() {
            log::debug!("main: Creating data directory");
            fs::create_dir_all(&data_dir)?;
        }

        registry::pull_devcontainer_index(&index_file)?;
        log::info!("Saved to {}", index_file.display());
    }

    if let Some(command) = args.command {
        if !index_file.exists() {
            // suggested user action
            log::error!("Missing devcontainer-index.json.\n\n\tRun `{} --pull-index`.\n", prog_name);
        }

        let index = registry::read_devcontainer_index(index_file)?;

        match command {
            Commands::Init (args) => init::init(&index, args)?,
            Commands::Inspect (args) => inspect::inspect(&index, args)?,
            Commands::List (args) => list::list(&index, args),
            Commands::Search (args) => search::search(&index, args)?,
        };
    }

    Ok(())
}

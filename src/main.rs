use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;

use clap::{Parser, Subcommand, ValueEnum};

mod configuration;
mod list;
mod registry;
mod search;

use configuration::DisplayPrompt;

#[derive(Clone, Debug, Default, ValueEnum)]
enum InspectDisplay {
    #[default]
    Table,
    Json,
    // Csv,
    // Yaml,
    // Toml,
    // SExpressions,
    // URL, // QueryString like name=Cody&age=32
}

/// Easily manage devcontainer configuration files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, arg_required_else_help = true)]
struct Args {
    /// Pull the index of features & templates
    #[arg(short, long)]
    pull_index: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create new devcontainer.
    Init {
        /// Avoid interactive prompts.
        #[arg(short, long)]
        non_interactive: bool,

        /// Write to ".devcontainer.json" when using an `image` type template.
        #[arg(short = 's', long)]
        attempt_single_file: bool,

        /// Strip comments from the generated devcontainer.json.
        #[arg(short, long)]
        remove_comments: bool,

        /// Reference to a Template in a supported OCI registry.
        #[arg(short, long, value_name = "OCI_REF")]
        template_id: Option<String>,

        /// Target workspace for the devcontainer configuration.
        #[arg(short, long, value_name = "DIRECTORY")]
        workspace_folder: Option<PathBuf>,
    },
    /// Display details of a specific template or feature.
    Inspect {
        /// The `id` to inspect.
        #[arg(value_name = "OCI_REF")]
        id: String,

        /// Format for displaying the results.
        #[arg(short, long, value_name = "FORMAT", default_value = "table")]
        display_as: InspectDisplay,
    },
    /// Overview of collections.
    List {
        /// Display a given collection, including features and templates.
        #[arg(short = 'C', long, value_name = "OCI_REF")]
        collection_id: Option<String>,
    },
    /// Text search the `id`, `keywords`, and `description` fields of templates or features.
    Search (search::SearchArgs),
}

fn program_name() -> io::Result<String> {
    let exe = env::current_exe()?;
    exe
    .file_name()
    .and_then(OsStr::to_str)
    .map(String::from)
    .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "Executable not a file path"))
}

fn data_directory<P: AsRef<Path>>(namespace: P) -> io::Result<PathBuf> {
    if let Some(path) = dirs::data_dir() {
        Ok(path.join(namespace))
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to determine a valid data directory"))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let prog_name = program_name()?;
    let data_dir = data_directory(&prog_name)?;
    let index_file = data_dir.join("devcontainer-index.json");

    if args.pull_index {
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        registry::pull_devcontainer_index(&index_file)?;
        println!("Saved to {}", index_file.display());
    }

    if let Some(command) = args.command {
        if !index_file.exists() {
            // suggested user action
            eprintln!("Missing devcontainer-index.json.\n\n\tRun `{} --pull-index`.\n", prog_name);
        }

        let index = registry::read_devcontainer_index(index_file)?;

        match command {
            Commands::Init { workspace_folder, .. } => {
                let _workspace = workspace_folder.map_or_else(env::current_dir, Ok)?;
                let name = "completions";
                let dev_option = registry::DevOption::default();
                let template_option = configuration::DevOptionPrompt::new(name, &dev_option);
                let value = template_option.display_prompt()?;

                println!("{}", value);
            },
            Commands::Inspect { .. } => (),
            Commands::List { collection_id } => {
                match collection_id {
                    Some(oci_reference) => {
                        match index.collections.iter().find(|&c| c.source_information.oci_reference == oci_reference) {
                            Some(collection) => {
                                list::collection_templates_and_features(&oci_reference, collection);
                            },
                            None => {
                                println!("No collection found by the given OCI Reference: {}", oci_reference);
                            },
                        }
                    },
                    None => list::overview_collections(&index),
                }
            },
            Commands::Search (args) => search::search(&index, args)?,
        };
    }

    Ok(())
}

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use clap::builder::PossibleValue;

mod registry;

#[derive(Clone, Debug, Default)]
enum CollectionName {
    #[default]
    Templates,
    Features,
}

impl ValueEnum for CollectionName {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::Templates, Self::Features]
    }

    fn to_possible_value(&self) -> Option<PossibleValue> {
        Some(match self {
            Self::Templates => PossibleValue::new("templates").alias("t"),
            Self::Features => PossibleValue::new("features").alias("f"),
        })
    }
}

#[derive(Clone, Debug, Default, ValueEnum)]
enum SearchFields {
    #[default]
    Id,
    Description,
    Keywords,
}

#[derive(Clone, Debug, Default, ValueEnum)]
enum SearchDisplay {
    #[default]
    Table,
    Json,
    // Csv,
    // Yaml,
    // Toml,
    // SExpressions,
    // URL, // QueryString like name=Cody&age=32
}

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

#[derive(Clone, Debug, Default, ValueEnum)]
enum ListType {
    #[default]
    Collections,
    Features,
    Templates,
}

/// Easily manage devcontainer configuration files.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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
        #[arg(default_value = "collections")]
        value: ListType,
    },
    /// Text search the `id`, `keywords`, and `description` fields of templates or features.
    Search {
        /// The keyword(s) to match.
        value: String,

        /// Match which section of the index.
        #[arg(short, long, default_value = "templates")]
        collection: CollectionName,

        /// Format for displaying the results.
        #[arg(short, long, value_name = "FORMAT", default_value = "table")]
        display_as: SearchDisplay,

        /// Match only within the given fields.
        #[arg(short, long)]
        fields: Option<Vec<SearchFields>>,
    },
}

fn program_name() -> io::Result<String> {
    let exe = env::current_exe()?;
    exe
    .file_name()
    .and_then(OsStr::to_str)
    .map(String::from)
    .ok_or(io::Error::new(io::ErrorKind::Other, "Executable not a file path"))
}

fn data_directory<P: AsRef<Path>>(namespace: P) -> io::Result<PathBuf> {
    if let Some(path) = dirs::data_dir() {
        Ok(path.join(namespace))
    } else {
        Err(io::Error::new(io::ErrorKind::InvalidData, "Unable to determine a valid data directory"))
    }
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let prog_name = program_name()?;
    let data_dir = data_directory(&prog_name)?;

    if args.pull_index {
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        registry::pull_devcontainer_index(&data_dir)?;
    }

    if let Some(command) = args.command {
        let _index = {
            let index_file = data_dir.join("devcontainer-index.json");
            let di = registry::read_devcontainer_index(index_file);

            if let Err(err) = &di {
                if err.kind() == io::ErrorKind::NotFound {
                    // suggested user action
                    eprintln!("Missing devcontainer-index.json.\n\n\tRun `{} --pull-index`.\n", prog_name);
                }
            }

            di
        }?;

        match command {
            Commands::Init { workspace_folder, .. } => {
                let _workspace = workspace_folder.map_or_else(env::current_dir, Ok)?;
                ()
            },
            Commands::Inspect { .. } => (),
            Commands::List { .. } => (),
            Commands::Search { .. } => (),
        };
    }

    Ok(())
}

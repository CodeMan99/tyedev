use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::fmt;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::result::Result;

use clap::{Parser, Subcommand, ValueEnum};
use clap::builder::PossibleValue;
use serde::{Deserialize, Serialize};

mod registry;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
enum CollectionCategory {
    #[default]
    Templates,
    Features,
}

impl fmt::Display for CollectionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Features => write!(f, "{}", "feature"),
            Self::Templates => write!(f, "{}", "template"),
        }
    }
}

impl ValueEnum for CollectionCategory {
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
    Name,
    Description,
    Keywords,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct SearchResult {
    /// The search result type, here for JSON type tagging. -- Reserves the right to transform this struct into an enum later.
    collection: CollectionCategory,
    id: String,
    version: String,
    name: String,
    description: Option<String>,
    keywords: Option<Vec<String>>,
}

impl From<&registry::Feature> for SearchResult {
    fn from(value: &registry::Feature) -> Self {
        SearchResult {
            collection: CollectionCategory::Features,
            id: value.id.clone(),
            version: value.version.clone(),
            name: value.name.clone(),
            description: value.description.clone(),
            keywords: value.keywords.clone(),
        }
    }
}

impl From<&registry::Template> for SearchResult {
    fn from(value: &registry::Template) -> Self {
        SearchResult {
            collection: CollectionCategory::Templates,
            id: value.id.clone(),
            version: value.version.clone(),
            name: value.name.clone(),
            description: value.description.clone(),
            keywords: value.keywords.clone(),
        }
    }
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
    Search {
        /// The keyword(s) to match.
        value: String,

        /// Match which section of the index.
        #[arg(short, long, default_value = "templates")]
        collection: CollectionCategory,

        /// Format for displaying the results.
        #[arg(short, long, value_name = "FORMAT", default_value = "table")]
        display_as: SearchDisplay,

        /// Match only within the given fields.
        #[arg(short, long)]
        fields: Option<Vec<SearchFields>>,

        /// Display deprecated results.
        #[arg(long)]
        include_deprecated: bool,
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

/// Take the lowercase `target` to check if it contains the lowercase `inside` value.
fn lowercase_contains<'t>(inside: &'t String) -> impl FnOnce(&'t String,) -> bool {
    move |target| target.to_lowercase().contains(inside.to_lowercase().as_str())
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
                ()
            },
            Commands::Inspect { .. } => (),
            Commands::List { collection_id } => {
                match collection_id {
                    Some(oci_reference) => {
                        match index.collections.iter().find(|&c| c.source_information.oci_reference == oci_reference) {
                            Some(collection) => {
                                let source_information = &collection.source_information;

                                println!("Name:          {}", &source_information.name);
                                println!("Maintainer:    {}", &source_information.maintainer);
                                println!("Contact:       {}", &source_information.contact);
                                println!("Repository:    {}", &source_information.repository);
                                println!("OCI Reference: {}", &source_information.oci_reference);

                                let search_results: Vec<SearchResult> = {
                                    let features = collection.features.iter().map(SearchResult::from);
                                    let templates = collection.templates.iter().map(SearchResult::from);
                                    features.chain(templates).collect()
                                };
                                let data: Vec<[String; 5]> =
                                    search_results.iter().enumerate().map(|(i, r)| {
                                        let description =
                                            r.description.as_ref()
                                            .and_then(|d| d.lines().next())
                                            .unwrap_or_default();
                                        [
                                            format!("{}", i + 1),
                                            format!("{}", r.collection),
                                            format!("{}", r.id.replace(&oci_reference, "~")),
                                            format!("{}", r.name),
                                            format!("{}", description),
                                        ]
                                    })
                                    .collect();
                                let mut table = ascii_table::AsciiTable::default();
                                table.column(0).set_align(ascii_table::Align::Right);
                                table.column(1).set_header("Type");
                                table.column(2).set_header("OCI Reference");
                                table.column(3).set_header("Name").set_max_width(40);
                                table.column(4).set_header("Description").set_max_width(75);
                                table.print(data);
                            },
                            None => {
                                println!("No collection found by the given OCI Reference: {}", oci_reference);
                            },
                        }
                    },
                    None => {
                        let mut table = ascii_table::AsciiTable::default();
                        table.column(0).set_header("Name");
                        table.column(1).set_header("OCI Reference");
                        table.column(2).set_header("Features").set_align(ascii_table::Align::Right);
                        table.column(3).set_header("Templates").set_align(ascii_table::Align::Right);
                        let result: Vec<[String; 4]> =
                            index.collections
                            .iter()
                            .map(|collection| {
                                [
                                    format!("{}", collection.source_information.name),
                                    format!("{}", collection.source_information.oci_reference),
                                    format!("{}", collection.features.len()),
                                    format!("{}", collection.templates.len()),
                                ]
                            })
                            .collect();
                        table.print(result);
                    },
                }
            },
            Commands::Search { value, collection, display_as, fields, include_deprecated } => {
                let search_fields = fields.unwrap_or_else(|| vec![SearchFields::Id, SearchFields::Name, SearchFields::Description]);
                let mut results: Vec<SearchResult> = Vec::new();

                match collection {
                    CollectionCategory::Features => {
                        index.collections
                        .iter()
                        .flat_map(|collection| collection.features.iter())
                        .for_each(|feature| {
                            for field in &search_fields {
                                let search_match = match field {
                                    SearchFields::Id => feature.id == value,
                                    SearchFields::Name => feature.name == value,
                                    SearchFields::Description => feature.description.as_ref().is_some_and(lowercase_contains(&value)),
                                    SearchFields::Keywords => feature.keywords.as_ref().is_some_and(|keywords| keywords.contains(&value)),
                                };

                                if search_match {
                                    let result = SearchResult::from(feature);
                                    results.push(result);
                                    break;
                                }
                            }
                        });
                    },
                    CollectionCategory::Templates => {
                        index.collections
                        .iter()
                        // There is one known collection that is deprecated, which is marked in the "maintainer" field.
                        .filter(|collection| include_deprecated || !collection.source_information.maintainer.to_lowercase().contains("deprecated"))
                        .flat_map(|collection| collection.templates.iter())
                        .for_each(|template| {
                            for field in &search_fields {
                                let search_match = match field {
                                    SearchFields::Id => template.id.to_lowercase().contains(value.to_lowercase().as_str()),
                                    SearchFields::Name => template.name.to_lowercase().contains(value.to_lowercase().as_str()),
                                    SearchFields::Description => template.description.as_ref().is_some_and(lowercase_contains(&value)),
                                    SearchFields::Keywords => template.keywords.as_ref().is_some_and(|keywords| keywords.contains(&value)),
                                };

                                if search_match {
                                    let result = SearchResult::from(template);
                                    results.push(result);
                                    break;
                                }
                            }
                        });
                    },
                }

                match display_as {
                    SearchDisplay::Table if results.is_empty() => println!("No results found"),
                    SearchDisplay::Table => {
                        let mut table = ascii_table::AsciiTable::default();
                        table.column(0).set_header("ID");
                        table.column(1).set_header("Version");
                        table.column(2).set_header("Name");
                        // table.column(3).set_header("Description");
                        let data: Vec<[&str; 3]> =
                            results
                            .iter()
                            .map(|r| [
                                r.id.as_str(),
                                r.version.as_str(),
                                r.name.as_str(),
                                // r.description.as_ref().and_then(|d| d.lines().next()).unwrap_or_default()
                            ])
                            .collect();
                        table.print(data);
                    },
                    SearchDisplay::Json => {
                        let json = serde_json::to_string(&results)?;
                        println!("{}", json);
                    },
                }
            },
        };
    }

    Ok(())
}

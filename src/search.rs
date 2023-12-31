use std::fmt;

use clap::builder::PossibleValue;
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::registry;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub enum CollectionCategory {
    #[default]
    Templates,
    Features,
}

impl fmt::Display for CollectionCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Features => write!(f, "feature"),
            Self::Templates => write!(f, "template"),
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
pub enum SearchFields {
    #[default]
    Id,
    Name,
    Description,
    Keywords,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchResult {
    /// The search result type, here for JSON type tagging. -- Reserves the right to transform this struct into an enum later.
    pub collection: CollectionCategory,
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub keywords: Option<Vec<String>>,
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

#[derive(Debug, Args)]
pub struct SearchArgs {
    /// The keyword(s) to match.
    value: String,

    /// Match which section of the index.
    #[arg(short, long, default_value = "templates")]
    collection: CollectionCategory,

    /// Format for displaying the results.
    #[arg(short, long, value_name = "FORMAT", default_value = "table")]
    display_as: SearchDisplay,

    /// Match only within the given fields.
    #[arg(short, long, value_name = "FIELD")]
    fields: Option<Vec<SearchFields>>,

    /// Display deprecated results.
    #[arg(long)]
    include_deprecated: bool,
}

/// Take the lowercase `target` to check if it contains the lowercase `inside` value.
fn lowercase_contains(inside: &str) -> impl FnOnce(&String) -> bool {
    let inside_lowercase = inside.to_lowercase();
    move |target| target.to_lowercase().contains(inside_lowercase.as_str())
}

trait SearchMatcher {
    fn is_match(&self, field: &SearchFields, value: &str) -> bool;
}

impl SearchMatcher for registry::Feature {
    fn is_match(&self, field: &SearchFields, value: &str) -> bool {
        match field {
            SearchFields::Id => lowercase_contains(value)(&self.id),
            SearchFields::Name => lowercase_contains(value)(&self.name),
            SearchFields::Description => self.description.as_ref().is_some_and(lowercase_contains(value)),
            SearchFields::Keywords => {
                (self.keywords.as_ref()).is_some_and(|keywords| keywords.contains(&String::from(value)))
            },
        }
    }
}

impl SearchMatcher for registry::Template {
    fn is_match(&self, field: &SearchFields, value: &str) -> bool {
        match field {
            SearchFields::Id => lowercase_contains(value)(&self.id),
            SearchFields::Name => lowercase_contains(value)(&self.name),
            SearchFields::Description => self.description.as_ref().is_some_and(lowercase_contains(value)),
            SearchFields::Keywords => {
                (self.keywords.as_ref()).is_some_and(|keywords| keywords.contains(&String::from(value)))
            },
        }
    }
}

fn search_match<T: SearchMatcher>(value: &T, text: &str, search_fields: &[SearchFields]) -> bool {
    search_fields.iter().any(|field| value.is_match(field, text))
}

pub fn search(
    index: &registry::DevcontainerIndex,
    SearchArgs {
        value: text,
        collection,
        display_as,
        fields,
        include_deprecated,
    }: SearchArgs,
) -> serde_json::error::Result<()> {
    log::debug!("search");

    let search_fields =
        fields.unwrap_or_else(|| vec![SearchFields::Id, SearchFields::Name, SearchFields::Description]);
    log::debug!("search: search_fields = {:?}", &search_fields);
    let results: Vec<SearchResult> = match collection {
        CollectionCategory::Features => {
            log::debug!("search: features");
            index
                .iter_features(include_deprecated)
                .filter_map(|feature| {
                    if search_match(feature, &text, &search_fields) {
                        Some(SearchResult::from(feature))
                    } else {
                        None
                    }
                })
                .collect()
        },
        CollectionCategory::Templates => {
            log::debug!("search: templates");
            index
                .iter_templates(include_deprecated)
                .filter_map(|template| {
                    if search_match(template, &text, &search_fields) {
                        Some(SearchResult::from(template))
                    } else {
                        None
                    }
                })
                .collect()
        },
    };

    match display_as {
        SearchDisplay::Table if results.is_empty() => println!("No results found"),
        SearchDisplay::Table => {
            let mut table = ascii_table::AsciiTable::default();
            table.column(0).set_header("ID");
            table.column(1).set_header("Version");
            table.column(2).set_header("Name");
            // table.column(3).set_header("Description");
            let data: Vec<[&str; 3]> = results
                .iter()
                .map(|r| {
                    [
                        r.id.as_str(),
                        r.version.as_str(),
                        r.name.as_str(),
                        // r.description.as_ref().and_then(|d| d.lines().next()).unwrap_or_default()
                    ]
                })
                .collect();
            table.print(data);
        },
        SearchDisplay::Json => {
            let json = serde_json::to_string(&results)?;
            println!("{json}");
        },
    }

    log::debug!("search: done");

    Ok(())
}

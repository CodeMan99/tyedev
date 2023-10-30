use clap::{Args, ValueEnum};

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum InspectDisplay {
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
pub struct InspectArgs {
    /// The `id` to inspect.
    #[arg(value_name = "OCI_REF")]
    id: String,

    /// Format for displaying the results.
    #[arg(short, long, value_name = "FORMAT", default_value = "table")]
    display_as: InspectDisplay,
}

pub fn inspect(InspectArgs { .. }: InspectArgs) {}

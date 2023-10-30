use std::env;
use std::error::Error;
use std::path::PathBuf;

use clap::Args;

use crate::configuration;
use crate::registry;

use configuration::DisplayPrompt;

#[derive(Debug, Args)]
pub struct InitArgs {
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
}

pub fn init(InitArgs { workspace_folder, .. }: InitArgs) -> Result<(), Box<dyn Error>> {
    let _workspace = workspace_folder.map_or_else(env::current_dir, Ok)?;
    let name = "completions";
    let dev_option = registry::DevOption::default();
    let template_option = configuration::DevOptionPrompt::new(name, &dev_option);
    let value = template_option.display_prompt()?;

    println!("{}", value);

    Ok(())
}

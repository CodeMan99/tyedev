use std::collections::HashMap;
use std::env;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::result::Result;
use std::str::FromStr;

use clap::Args;
use inquire::{autocompletion::Replacement, Autocomplete, Confirm, CustomUserError, Select, Text};
use regex::bytes::{Captures, Regex};
use serde_json::{self, Map, Value};
use tar::{self, Archive, Builder, EntryType, Header};

use crate::oci_ref::OciReference;
use crate::registry::{self, DevOption, StringDevOption};

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Avoid interactive prompts.
    #[arg(short = 'z', long)]
    non_interactive: bool,

    /// Write to ".devcontainer.json" when using an `image` type template.
    #[arg(short = 's', long)]
    attempt_single_file: bool,

    /// Strip comments from the generated devcontainer.json.
    #[arg(short, long)]
    remove_comments: bool,

    /// Reference to a Template in a supported OCI registry.
    #[arg(short, long, value_name = "OCI_REF")]
    template_id: Option<OciReference>,

    /// Add the given features, may specify more than once.
    #[arg(short = 'f', long, value_name = "OCI_REF")]
    include_features: Option<Vec<OciReference>>,

    /// Include deprecated results when searching.
    #[arg(long)]
    include_deprecated: bool,

    /// Target workspace for the devcontainer configuration.
    #[arg(short, long, value_name = "DIRECTORY")]
    workspace_folder: Option<PathBuf>,
}

async fn get_feature(
    index: &registry::DevcontainerIndex,
    feature_ref: &OciReference,
) -> anyhow::Result<registry::Feature> {
    log::debug!("get_feature");

    match index.get_feature(&feature_ref.id()) {
        Some(feature) => Ok(feature.clone()),
        None => pull_feature_configuration(feature_ref).await,
    }
}

async fn pull_feature_configuration(feature_ref: &OciReference) -> anyhow::Result<registry::Feature> {
    log::debug!("pull_feature_configuration");
    let bytes = registry::pull_archive_bytes(feature_ref).await?;
    let mut archive = Archive::new(bytes.as_slice());
    let entries = archive.entries()?;

    for entry in entries {
        let mut entry = entry?;
        let filename = entry.path()?;
        let filename = filename.to_str();

        if filename.is_some_and(|p| p.ends_with("devcontainer-feature.json")) {
            let size = entry.size() as usize;
            let mut data: Vec<u8> = Vec::with_capacity(size);
            entry.read_to_end(&mut data)?;
            let feature: registry::Feature = serde_json::from_slice(data.as_slice())?;

            log::debug!("pull_feature_configuration: read {size} bytes");

            return Ok(feature);
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No devcontainer-feature.json found in archive",
    ))?
}

#[derive(Clone, Debug, PartialEq, Default)]
struct DevOptionProposalsAutocomplete(Vec<String>);

impl DevOptionProposalsAutocomplete {
    fn new(default_value: &String, values: &[String]) -> DevOptionProposalsAutocomplete {
        if default_value.is_empty() || values.contains(default_value) {
            DevOptionProposalsAutocomplete(values.into())
        } else {
            let mut all_values: Vec<String> = values.into();
            all_values.insert(0, default_value.clone());
            DevOptionProposalsAutocomplete(all_values)
        }
    }
}

impl Autocomplete for DevOptionProposalsAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        let DevOptionProposalsAutocomplete(proposals) = self;
        let input_lower = input.to_lowercase();
        let suggestions = proposals
            .iter()
            .filter_map(|s| {
                if s.to_lowercase().starts_with(&input_lower) {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect();
        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        Ok(highlighted_suggestion.or_else(|| {
            let suggestions = self.get_suggestions(input).ok()?;
            if let [suggestion] = suggestions.as_slice() {
                Some(suggestion.clone())
            } else {
                None
            }
        }))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DevOptionPromptValue {
    String(String),
    Boolean(bool),
}

impl Display for DevOptionPromptValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevOptionPromptValue::Boolean(value) => write!(f, "{value}"),
            DevOptionPromptValue::String(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DevOptionPrompt<'t> {
    inner: &'t DevOption,
    name: &'t str,
}

impl<'t> DevOptionPrompt<'t> {
    pub fn new(name: &'t str, dev_option: &'t DevOption) -> DevOptionPrompt<'t> {
        DevOptionPrompt {
            inner: dev_option,
            name,
        }
    }

    fn display_prompt(&self) -> anyhow::Result<DevOptionPromptValue> {
        let dev_option = self.inner;
        let default = dev_option.configured_default();

        match dev_option {
            DevOption::Boolean { description, .. } => {
                let message = description
                    .as_ref()
                    .map_or_else(|| format!("Include {}?", self.name), |s| s.clone());
                let default_value = bool::from_str(&default)?;
                let result = Confirm::new(&message).with_default(default_value).prompt()?;
                let value = DevOptionPromptValue::Boolean(result);

                Ok(value)
            },
            DevOption::String(StringDevOption::EnumValues {
                description, r#enum, ..
            }) => {
                let message = description
                    .as_ref()
                    .map_or_else(|| format!("Choose value for {}:", self.name), |s| s.clone());
                let options = r#enum.iter().collect();
                let start = r#enum.iter().position(|s| *s == default).unwrap_or_default();
                let result = Select::new(&message, options).with_starting_cursor(start).prompt()?;
                let value = DevOptionPromptValue::String(result.clone());

                Ok(value)
            },
            DevOption::String(StringDevOption::Proposals {
                description, proposals, ..
            }) => {
                let message = description
                    .as_ref()
                    .map_or_else(|| format!("What value for {}?", self.name), |s| s.clone());
                let text_prompt = if let Some(values) = proposals.as_ref().filter(|&p| !p.is_empty()) {
                    let autocomplete = DevOptionProposalsAutocomplete::new(&default, values);

                    Text::new(&message)
                        .with_default(&default)
                        .with_autocomplete(autocomplete)
                } else {
                    Text::new(&message).with_default(&default)
                };
                let result = text_prompt.prompt()?;
                let value = DevOptionPromptValue::String(result);

                Ok(value)
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
struct FeaturesAutocomplete(Vec<String>);

impl FeaturesAutocomplete {
    fn new(index: &registry::DevcontainerIndex, include_deprecated: bool) -> Self {
        let inner = index
            .iter_features(include_deprecated)
            .map(|feature| feature.id.clone())
            .collect();
        FeaturesAutocomplete(inner)
    }
}

impl inquire::Autocomplete for FeaturesAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        let FeaturesAutocomplete(proposals) = self;
        let suggestions = proposals
            .iter()
            .filter_map(|feature_id| {
                if feature_id.contains(input) {
                    Some(feature_id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<String>>();
        Ok(suggestions)
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(highlighted_suggestion.or_else(|| {
            let suggestions = self.get_suggestions(input).ok()?;
            if let [suggestion] = suggestions.as_slice() {
                Some(suggestion.clone())
            } else {
                None
            }
        }))
    }
}

#[derive(Clone, Debug, Default)]
struct FeatureEntryBuilder {
    features: HashMap<String, Value>,
}

impl FeatureEntryBuilder {
    fn new() -> Self {
        log::debug!("FeatureEntryBuilder::new");
        FeatureEntryBuilder {
            features: HashMap::new(),
        }
    }

    fn use_prompt_values(&mut self, feature: &registry::Feature) -> anyhow::Result<()> {
        log::debug!("FeatureEntryBuilder::use_prompt_values");
        let key = format!("{}:{}", feature.id, feature.major_version);
        let value = {
            let mut inner = Map::new();

            if let Some(options) = &feature.options {
                for (name, dev_option) in options {
                    let prompt = DevOptionPrompt::new(name, dev_option);
                    let prompt_value = prompt.display_prompt()?;

                    // TODO consider using inquire::{PromptType}::prompt_skippable instead.
                    if prompt_value.to_string() == dev_option.configured_default() {
                        continue;
                    }

                    let value = match prompt_value {
                        DevOptionPromptValue::Boolean(b) => serde_json::to_value(b),
                        DevOptionPromptValue::String(s) => serde_json::to_value(s),
                    }?;

                    inner.insert(name.clone(), value);
                }
            }

            Value::Object(inner)
        };

        self.features.insert(key, value);

        Ok(())
    }

    fn use_default_values(&mut self, feature: &registry::Feature) {
        log::debug!("FeatureEntryBuilder::use_default_values");
        let key = format!("{}:{}", feature.id, feature.major_version);
        let value = Value::Object(Map::default());

        self.features.insert(key, value);
    }

    fn as_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self.features.clone())
    }

    fn len(&self) -> usize {
        self.features.len()
    }
}

#[derive(Debug, Default)]
struct TemplateBuilder {
    config: Option<registry::Template>,
    context: HashMap<String, String>,
    features: FeatureEntryBuilder,
    archive_bytes: Vec<u8>,
}

impl TemplateBuilder {
    async fn new(template_ref: &OciReference, config: Option<registry::Template>) -> anyhow::Result<Self> {
        log::debug!("TemplateBuilder::new");
        let archive_bytes = registry::pull_archive_bytes(template_ref).await?;
        let template_archive = TemplateBuilder {
            config,
            context: HashMap::new(),
            features: FeatureEntryBuilder::new(),
            archive_bytes,
        };

        Ok(template_archive)
    }

    fn as_archive(&self) -> Archive<&[u8]> {
        Archive::new(self.archive_bytes.as_slice())
    }

    fn replace_config(&mut self) -> std::io::Result<()> {
        log::debug!("TemplateBuilder::replace_config");
        let mut tar = self.as_archive();
        let entries = tar.entries()?;

        for entry in entries {
            let mut entry = entry?;
            let path = entry.path()?;

            if path.to_str().is_some_and(|p| p.ends_with("devcontainer-template.json")) {
                let size = entry.size() as usize;
                let mut data: Vec<u8> = Vec::with_capacity(size);
                entry.read_to_end(&mut data)?;
                let config = serde_json::from_slice(data.as_slice())?;

                self.config.replace(config);
                log::debug!("TemplateBuilder::replace_config: read {size} bytes");

                return Ok(());
            }
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "The devcontainer-template.json file was not found in the archive",
        ))?
    }

    fn use_prompt_values(&mut self) -> anyhow::Result<()> {
        log::debug!("TemplateBuilder::use_prompt_values");
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| io::Error::other("Missing configuration"))?;

        if let Some(options) = &config.options {
            self.context.clear();

            for (name, template_option) in options {
                let dev_prompt = DevOptionPrompt::new(name, template_option);
                let value = dev_prompt.display_prompt()?;
                self.context.insert(name.clone(), value.to_string());
            }
        }

        Ok(())
    }

    fn use_default_values(&mut self) -> std::io::Result<()> {
        log::debug!("TemplateBuilder::use_default_values");
        let config = self
            .config
            .as_ref()
            .ok_or_else(|| io::Error::other("Missing configuration"))?;

        if let Some(options) = &config.options {
            let all_defaults = options
                .iter()
                .map(|(name, template_option)| (name.clone(), template_option.configured_default()))
                .collect::<HashMap<String, String>>();

            self.context.clear();
            self.context.clone_from(&all_defaults);
        }

        Ok(())
    }

    fn is_single_file_eligible(&self) -> bool {
        if let Some(template) = self.config.as_ref() {
            if let Some(template_type) = template.r#type.as_ref() {
                return match template_type {
                    registry::TemplateType::DockerCompose => {
                        log::warn!(
                            "Skipping --attempt-single-file as the selected template includes a docker-compose.yml"
                        );
                        false
                    },
                    registry::TemplateType::Dockerfile => {
                        log::warn!("Skipping --attempt-single-file as the selected template includes a Dockerfile");
                        false
                    },
                    // Most image templates have exactly four files: .devcontainer/devcontainer.json, devcontainer-feature.json, NOTES.md, README.md
                    registry::TemplateType::Image if template.file_count.is_some_and(|count| count > 4) => {
                        log::warn!("Skipping --attempt-single-file as the template likely contains more than just a devcontainer.json file");
                        false
                    },
                    registry::TemplateType::Image => true,
                };
            }
        }

        false
    }

    fn apply_context_and_features(&mut self, attempt_single_file: bool, workspace: &Path) -> anyhow::Result<()> {
        log::debug!("TemplateBuilder::apply_context_and_features");
        let template_option_re = Regex::new(r"\$\{templateOption:\s*(?<name>\w+)\s*\}")?;
        let apply_context = |captures: &Captures| -> &[u8] {
            let name = &captures["name"];
            let name = std::str::from_utf8(name).ok();
            match name.and_then(|key| self.context.get(key)) {
                Some(value) => {
                    log::debug!(
                        "TemplateBuilder::apply_context_and_features: Replacing ${{templateOption:{}}} with \"{}\"",
                        name.unwrap_or_default(),
                        value
                    );
                    value.as_bytes()
                },
                None => {
                    log::warn!("No value provided for ${{templateOption:{}}}", name.unwrap_or_default());
                    b""
                },
            }
        };
        let mut archive = self.as_archive();
        let entries = archive.entries()?;
        let template_skip = ["NOTES.md", "README.md", "devcontainer-template.json"];

        for entry in entries {
            let mut entry = entry?;
            let relative_path = entry.path()?;
            let mut filename = workspace.join(relative_path);

            if template_skip.iter().any(|&name| filename.ends_with(name)) {
                log::debug!(
                    "TemplateBuilder::apply_context_and_features: Skipping template file: {}",
                    filename.display()
                );
                continue;
            }

            match entry.header().entry_type() {
                EntryType::Directory => {
                    log::info!("Creating directory: {}", filename.display());
                    fs::create_dir_all(&filename)?;
                },
                EntryType::Regular | EntryType::Continuous => {
                    log::info!("Reading file from template archive: {}", filename.display());

                    let mut bytes: Vec<u8> = Vec::with_capacity(entry.size() as usize);

                    entry.read_to_end(&mut bytes)?;

                    let with_context = template_option_re.replace_all(bytes.as_mut_slice(), apply_context);
                    let dc_filename1 = ".devcontainer/devcontainer.json";
                    let dc_filename2 = ".devcontainer.json";

                    if filename.ends_with(dc_filename1) || filename.ends_with(dc_filename2) {
                        if attempt_single_file && self.is_single_file_eligible() {
                            filename = workspace.join(".devcontainer.json");
                        }

                        if self.features.len() > 0 {
                            let mut bytes: Vec<u8> = Vec::new();
                            bytes.write_all(&with_context)?;
                            let mut value: Value = serde_jsonc::from_slice(bytes.as_slice())?;
                            let devcontainer = value.as_object_mut().ok_or_else(|| {
                                io::Error::new(io::ErrorKind::InvalidData, "Format of devcontainer.json is invalid")
                            })?;
                            match devcontainer.get_mut("features").and_then(|f| f.as_object_mut()) {
                                Some(features) => features.extend(self.features.features.clone()),
                                None => {
                                    let features_value = self.features.as_value()?;
                                    devcontainer.insert("features".into(), features_value);
                                },
                            }
                            log::warn!("Comments have been stripped from devcontainer.json");
                            log::info!("Writing to {}", filename.display());
                            let file = File::create(filename)?;
                            serde_json_pretty::to_writer_with_tabs(file, &value)?;
                        } else {
                            log::info!("Writing to {}", filename.display());
                            let mut file = File::create(filename)?;
                            file.write_all(&with_context)?;
                        }
                    } else {
                        log::info!("Writing to {}", filename.display());
                        let mut file = File::create(filename)?;
                        file.write_all(&with_context)?;
                    }
                },
                _ => (),
            }
        }

        log::debug!("TemplateBuilder::apply_context_and_features: done");

        Ok(())
    }

    fn create_empty_start_point() -> anyhow::Result<Self> {
        let template_value = serde_json::json!({
            "id": "tyedev-base-template",
            "version": "1.0.0",
            "name": "Base Template (tyedev)",
            "options": {
                "imageVariant": {
                    "type": "string",
                    "default": "jammy",
                    "proposals": [
                        "bookworm",
                        "bullseye",
                        "jammy",
                        "focal"
                    ]
                }
            },
            "type": "image",
            "fileCount": 2,
            "owner": "CodeMan99"
        });
        let tar_blocksize = 512;
        // 3 header blocks, 2 content blocks, 2 zero blocks
        let tar_chunks = 7;
        let mut builder = Builder::new(Vec::with_capacity(tar_blocksize * tar_chunks));
        let mtime = {
            let unix_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?;
            unix_time.as_secs()
        };

        let create_directory_header = |path: &str| -> io::Result<Header> {
            let mut header = Header::new_gnu();
            header.set_entry_type(EntryType::Directory);
            header.set_path(path)?;
            header.set_size(0);
            header.set_mtime(mtime);
            header.set_mode(0o775);
            header.set_uid(1000);
            header.set_gid(1000);
            header.set_cksum();
            Ok(header)
        };

        let create_file_header = |size| {
            let mut header = Header::new_gnu();
            header.set_entry_type(EntryType::Regular);
            header.set_size(size);
            header.set_mtime(mtime);
            header.set_mode(0o664);
            header.set_uid(1000);
            header.set_gid(1000);
            header
        };

        let dot_devcontainer_dir = create_directory_header(".devcontainer/")?;
        builder.get_mut().write_all(dot_devcontainer_dir.as_bytes())?;

        let dot_devcontainer_json: &[u8] = b"{\n\t\"name\": \"tyedev default\",\n\t\"image\": \"mcr.microsoft.com/devcontainers/base:${templateOption:imageVariant}\"\n}\n";
        let mut header_devcontainer_json = create_file_header(dot_devcontainer_json.len() as u64);
        builder.append_data(
            &mut header_devcontainer_json,
            ".devcontainer/devcontainer.json",
            dot_devcontainer_json,
        )?;

        let devcontainer_template_json = serde_json::to_string_pretty(&template_value)?;
        let mut header_template_json = create_file_header(devcontainer_template_json.len() as u64);
        builder.append_data(
            &mut header_template_json,
            "devcontainer-template.json",
            devcontainer_template_json.as_bytes(),
        )?;

        let archive_bytes = builder.into_inner()?;

        #[cfg(test)]
        {
            let tmp = env::temp_dir();
            let mut file = File::create(tmp.join("devcontainer-template-tyedev-default.tar"))?;
            file.write_all(&archive_bytes)?;
        }

        let tb = TemplateBuilder {
            config: serde_json::from_value(template_value).ok(),
            context: HashMap::default(),
            features: FeatureEntryBuilder::default(),
            archive_bytes,
        };

        Ok(tb)
    }
}

mod serde_json_pretty {
    use serde::Serialize;
    use serde_json::{error::Result, ser::PrettyFormatter, Serializer};
    use std::io::Write;

    /// This is the same as `serde_json::to_writer_pretty` except with use of tabs for indentation.
    pub fn to_writer_with_tabs<W: Write, V: ?Sized + Serialize>(writer: W, value: &V) -> Result<()> {
        let formatter = PrettyFormatter::with_indent(b"\t");
        let mut serializer = Serializer::with_formatter(writer, formatter);
        value.serialize(&mut serializer)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn test_to_writer_with_tabs() -> Result<()> {
            let value = serde_json::json!({"test": {"deep": 1}});
            let mut vec: Vec<u8> = Vec::new();
            to_writer_with_tabs(&mut vec, &value)?;
            let bytes = vec.as_slice();
            assert_eq!(bytes, b"{\n\t\"test\": {\n\t\t\"deep\": 1\n\t}\n}");
            Ok(())
        }
    }
}

#[derive(Debug, PartialEq)]
enum PromptEntryAction {
    Existing,
    Enter,
    Empty,
}

impl Display for PromptEntryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Existing => write!(f, "Pick existing template"),
            Self::Enter => write!(f, "Enter known template OCI reference"),
            Self::Empty => write!(f, "Start from scratch"),
        }
    }
}

pub async fn init(
    index: &registry::DevcontainerIndex,
    InitArgs {
        non_interactive,
        attempt_single_file,
        remove_comments: _,
        template_id,
        include_features,
        include_deprecated,
        workspace_folder,
    }: InitArgs,
) -> anyhow::Result<()> {
    log::debug!("init");
    // Do this evaluation of the `env` first so that it can error early.
    let workspace = workspace_folder.map_or_else(env::current_dir, Ok)?;

    /*
     * Done        1(a). What template are we starting with?
     * Done        1(b). Start with an empty, image-based devcontainer.json.
     * Done        2(a). Pick values for any temapltes options.
     * Done        2(b). Replace `${templateOption:name}` placeholders. Reminder that this is *not* limited to
     *                   devcontainer.json and may appear in any of the template's files from the tar archive.
     *
     *                       const pattern = /\${templateOption:\s*(\w+?)\s*}/g; // ${templateOption:XXXX}
     *
     * Done        3(a). Prompt for features loop?
     * Done        3(b). Search for feature.
     * Done        3(c). Pick values for any feature options.
     * Done        3(d). Edit devcontainer.json.
     *             4(a). Display the resulting devcontainer.json.
     *             4(b). Prompt loop to (A)ccept, (E)dit, (R)estart, or (Q)uit
     * Done           5. Write files to disk.
     */
    let mut template_builder: TemplateBuilder = match &template_id {
        Some(template_ref) => {
            let id = template_ref.id();
            let template = index.get_template(&id);

            TemplateBuilder::new(template_ref, template.cloned()).await?
        },
        None if non_interactive => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Must provide --template-id in non-interactive mode",
        ))?,
        None => {
            let start_point = inquire::Select::new(
                "Choose a starting point:",
                vec![
                    PromptEntryAction::Existing,
                    PromptEntryAction::Enter,
                    PromptEntryAction::Empty,
                ],
            )
            .prompt()?;

            match start_point {
                PromptEntryAction::Existing => {
                    let template_ids = index
                        .iter_templates(include_deprecated)
                        .map(|template| template.id.clone())
                        .collect();
                    let template_id =
                        inquire::Select::new("Pick existing template from the index:", template_ids).prompt()?;
                    let template_ref = template_id.parse()?;
                    let template = index.get_template(&template_id);
                    TemplateBuilder::new(&template_ref, template.cloned()).await?
                },
                PromptEntryAction::Enter => {
                    let template_id = inquire::Text::new("Enter template by providing the OCI reference:").prompt()?;
                    let template_ref = template_id.parse()?;
                    let template = index.get_template(&template_id);
                    TemplateBuilder::new(&template_ref, template.cloned()).await?
                },
                PromptEntryAction::Empty => TemplateBuilder::create_empty_start_point()?,
            }
        },
    };

    let is_version_tag = template_id
        .as_ref()
        .is_some_and(|oci_ref| oci_ref.tag_name() != "latest");

    if is_version_tag || template_builder.config.is_none() {
        template_builder.replace_config()?;
    }

    if non_interactive {
        template_builder.use_default_values()?;

        if let Some(feature_refs) = include_features {
            for feature_ref in feature_refs {
                let feature = get_feature(index, &feature_ref).await?;
                log::info!("Adding feature: {}", feature_ref.id());
                template_builder.features.use_default_values(&feature);
            }
        }
    } else {
        template_builder.use_prompt_values()?;

        if let Some(feature_refs) = include_features {
            for feature_ref in feature_refs {
                let feature = get_feature(index, &feature_ref).await?;
                println!("Adding feature: {}", feature_ref.id());
                template_builder.features.use_prompt_values(&feature)?;
            }
        }

        loop {
            let next = inquire::Confirm::new("Add a feature?").prompt()?;

            if next {
                let features_autocomplete = FeaturesAutocomplete::new(index, include_deprecated);
                let input = inquire::Text::new("Choose or enter feature id (OCI REF):")
                    .with_autocomplete(features_autocomplete)
                    .prompt()?;
                let feature_ref: OciReference = input.parse()?;
                let feature = get_feature(index, &feature_ref).await?;

                template_builder.features.use_prompt_values(&feature)?;
            } else {
                break;
            }
        }
    }

    template_builder.apply_context_and_features(attempt_single_file, &workspace)?;
    log::debug!("init: done");

    Ok(())
}

// TODO these are more *proof of concept* than actual tests...
#[cfg(test)]
mod tests {
    use super::{FeatureEntryBuilder, TemplateBuilder};
    use serde_json::{self, Map, Value};

    #[test]
    fn test_feature_entry_builder_as_value() -> serde_json::error::Result<()> {
        let mut feature_entry_builder = FeatureEntryBuilder::default();

        let git_id = "ghcr.io/devcontainers/git:1";
        let git_options = Value::Object(Map::default());
        feature_entry_builder.features.insert(git_id.to_owned(), git_options);

        let dind_id = "ghcr.io/devcontainers/docker-in-docker:2";
        let dind_options = serde_json::json!({"moby": false});
        feature_entry_builder.features.insert(dind_id.to_owned(), dind_options);

        let value = feature_entry_builder.as_value()?;
        let json_str = serde_json::to_string_pretty(&value)?;

        println!("{{\"features\": {json_str}\n}}");

        Ok(())
    }

    #[test]
    fn test_create_empty_start_point() -> anyhow::Result<()> {
        let _template_builder = TemplateBuilder::create_empty_start_point()?;
        Ok(())
    }
}

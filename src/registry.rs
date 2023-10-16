use std::fs::File;
use std::io::{Error, ErrorKind};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use serde::{Deserialize, Serialize};

// PartialOrd, Hash, Eq, Ord
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerMountType {
    #[default]
    Bind,
    Volume,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct DockerMount {
    pub source: String,
    pub target: String,
    pub r#type: DockerMountType,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LifecycleHook {
    Single(String),
    Multiple(Vec<String>),
    Named(HashMap<String, Box<LifecycleHook>>),
}

impl Default for LifecycleHook {
    fn default() -> Self {
        LifecycleHook::Single(String::new())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInformation {
    pub name: String,
    pub maintainer: String,
    pub contact: String,
    pub repository: String,
    pub oci_reference: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum BooleanDefaultType {
    String(String),
    Boolean(bool),
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringDevOption {
    Proposals {
        default: Option<String>, // this field is actually required, but there are violations out there that need to be loaded.
        description: Option<String>,
        proposals: Option<Vec<String>>,
    },
    EnumValues {
        default: String,
        description: Option<String>,
        r#enum: Vec<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DevOption {
    Boolean {
        default: BooleanDefaultType, // this is sometimes a bool.
        description: Option<String>,
    },
    String (StringDevOption),
}

impl Default for DevOption {
    fn default() -> Self {
        DevOption::Boolean {
            default: BooleanDefaultType::String(String::from("true")),
            description: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "documentationURL")]
    pub documentation_url: Option<String>,
    #[serde(rename = "licenseURL")]
    pub license_url: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub options: Option<HashMap<String, DevOption>>,
    pub container_env: Option<HashMap<String, String>>,
    pub privileged: Option<bool>,
    pub init: Option<bool>,
    pub cap_add: Option<Vec<String>>,
    pub security_opt: Option<Vec<String>>,
    pub entrypoint: Option<String>,
    // pub customizations: HashMap<String, String>, // this type is wrong - it is a dynamic field
    pub installs_after: Option<Vec<String>>,
    pub lecagy_ids: Option<Vec<String>>,
    pub deprecated: Option<bool>,
    pub mounts: Option<Vec<DockerMount>>,
    pub on_create_command: Option<LifecycleHook>,
    pub update_content_command: Option<LifecycleHook>,
    pub post_create_command: Option<LifecycleHook>,
    pub post_start_command: Option<LifecycleHook>,
    pub post_attach_command: Option<LifecycleHook>,
    pub owner: String,
    pub major_version: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateType {
    #[default]
    Image,
    Dockerfile,
    DockerCompose,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "documentationURL")]
    pub documentation_url: Option<String>,
    #[serde(rename = "licenseURL")]
    pub license_url: Option<String>,
    pub options: Option<HashMap<String, DevOption>>,
    pub platforms: Option<Vec<String>>,
    pub publisher: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub r#type: Option<TemplateType>,
    pub file_count: Option<i32>,
    pub feature_ids: Option<Vec<String>>,
    pub owner: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub source_information: SourceInformation,
    pub features: Vec<Feature>,
    pub templates: Vec<Template>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct DevcontainerIndex {
    pub collections: Vec<Collection>,
}

/// Execute the `oras` binary to pull "ghcr.io/devcontainers/index:latest" which will output "devcontainer-index.json".
pub fn pull_devcontainer_index<P: AsRef<Path>>(current_dir: P) -> Result<(), Error> {
    Command::new("oras")
    .args(["pull", "ghcr.io/devcontainers/index:latest"])
    .current_dir(current_dir)
    .status()
    .and_then(|exit_status| {
        if exit_status.success() {
            Ok(())
        } else {
            Err(Error::new(ErrorKind::Interrupted, "Non-zero exit code received from `oras` subprocess."))
        }
    })
}

pub fn read_devcontainer_index<P: AsRef<Path>>(filename: P) -> Result<DevcontainerIndex, Error> {
    let file = File::open(filename)?;
    let index = serde_json::from_reader(file)?;

    Ok(index)
}

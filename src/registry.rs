use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::collections::HashMap;
use std::path::Path;

use oci_spec::image::MediaType;
use ocipkg::{Digest, ImageName, distribution};
use serde_json::Value as JsonValue;
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
    EnumValues {
        default: String,
        description: Option<String>,
        r#enum: Vec<String>,
    },
    Proposals {
        default: Option<String>, // this field is actually required, but there are violations out there that need to be loaded.
        description: Option<String>,
        proposals: Option<Vec<String>>,
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

impl DevOption {
    pub fn configured_default(&self) -> String {
        match self {
            DevOption::Boolean { default, .. } => {
                match default {
                    BooleanDefaultType::String(s) => s.clone(),
                    BooleanDefaultType::Boolean(b) => format!("{}", b),
                }
            },
            DevOption::String(StringDevOption::Proposals { default, proposals, .. }) => {
                // Reminder that `default` is not actually optional. This is just covering mistakes from collection maintainers.
                default.clone()
                .or_else(||
                    proposals.as_ref()
                    .and_then(|p| p.first().cloned())
                )
                .unwrap_or_default()
            },
            DevOption::String(StringDevOption::EnumValues { default, .. }) => {
                default.clone()
            },
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

/// Pull OCI Artifact "ghcr.io/devcontainers/index:latest" and download the JSON layer to the given filename.
pub fn pull_devcontainer_index<P: AsRef<Path>>(filename: P) -> Result<(), Box<dyn std::error::Error>> {
    let image_name = ImageName::parse("ghcr.io/devcontainers/index:latest")?;
    let mut client = distribution::Client::try_from(&image_name)?;
    let layer = distribution::get_image_layer(&mut client, &image_name, |media_type| {
        match media_type {
            MediaType::Other(other_type) => other_type == "application/vnd.devcontainers.index.layer.v1+json",
            _ => false,
        }
    })?;
    let digest = Digest::new(layer.digest())?;
    let blob = client.get_blob(&digest)?;
    let mut file = File::create(filename)?;

    file.write_all(&blob[..])?;

    Ok(())
}

pub fn pull_template<P: AsRef<Path>>(folder: P, image_name: &ImageName) -> Result<(), Box<dyn std::error::Error>> {
    let mut client = distribution::Client::try_from(image_name)?;
    let layer = distribution::get_image_layer(&mut client, image_name, |media_type| {
        match media_type {
            MediaType::Other(other_type) => other_type == "application/vnd.devcontainers.layer.v1+tar",
            _ => false,
        }
    })?;
    // let filename =
    //     layer.annotations()
    //     .to_owned()
    //     .and_then(|annotations| annotations.get("org.opencontainers.image.title").map(|title| title.to_owned()))
    //     .unwrap_or_else(|| String::from("template.tar"));
    let digest = Digest::new(layer.digest())?;
    let blob = client.get_blob(&digest)?;
    let mut archive = tar::Archive::new(blob.as_slice());

    // TODO determine what files are actually needed.
    archive.unpack(folder)?;

    Ok(())
}

/// Read and parse the given filename.
pub fn read_devcontainer_index<P: AsRef<Path>>(filename: P) -> Result<DevcontainerIndex, Error> {
    let file = File::open(filename)?;
    let json_value: JsonValue = serde_json::from_reader(file)?;
    let collections: Vec<Collection> =
        json_value
        .as_object()
        .and_then(|obj_map| obj_map.get("collections"))
        .and_then(|collections_value| collections_value.as_array())
        .map_or_else(
            || Err(Error::new(ErrorKind::InvalidData, "Unexpected json shape")),
            |arr| {
                let parsed =
                    arr
                    .iter()
                    // TODO: Skip errors of a single feature or template, not the entire collection
                    .filter_map(|value| {
                        match serde_json::from_value::<Collection>(value.to_owned()) {
                            Ok(collection) => Some(collection),
                            Err(_) => {
                                // TODO: parse the collection fields so that source_information can be displayed here.
                                eprintln!("WARNING: Skipping collection due to parsing error");
                                None
                            },
                        }
                    })
                    .collect();

                Ok(parsed)
            }
        )?;

    Ok(DevcontainerIndex { collections })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_template() -> Result<(), Box<dyn std::error::Error>> {
        let tmpdir = tempfile::tempdir()?;
        let inner = |folder| {
            let image_name = ImageName::parse("ghcr.io/devcontainers/templates/go:latest")?;
            pull_template(folder, &image_name)
        };
        let result = inner(&tmpdir);

        std::fs::remove_dir_all(tmpdir)?;

        result
    }
}

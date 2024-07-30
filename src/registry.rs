use std::collections::HashMap;
use std::fmt::{self, Display};
use std::fs::{self, File};
use std::io::{Error, ErrorKind, Write};
use std::ops::Not;
use std::path::Path;

use anyhow::{Context, Result};
use ocipkg::distribution::MediaType;
use ocipkg::image::{Artifact, Image};
use ocipkg::{Digest, ImageName};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::oci_ref::OciReference;

// PartialOrd, Hash, Eq, Ord
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DockerMountType {
    #[default]
    Bind,
    Volume,
}

impl Display for DockerMountType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DockerMountType::Bind => write!(f, "bind"),
            DockerMountType::Volume => write!(f, "volume"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct DockerMount {
    pub source: String,
    pub target: String,
    pub r#type: DockerMountType,
}

impl Display for DockerMount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "source={}, target={}, type={}",
            self.source, self.target, self.r#type
        )
    }
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

impl Display for LifecycleHook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LifecycleHook::Single(value) => write!(f, "{value}"),
            LifecycleHook::Multiple(values) => write!(f, "{}", values.join(", ")),
            LifecycleHook::Named(values) => {
                values.iter().enumerate().try_for_each(|(index, (key, value))| {
                    let join = if index == 0 { "" } else { "; " };
                    match value.as_ref() {
                        LifecycleHook::Single(v) => write!(f, "{join}{key}={v}"),
                        LifecycleHook::Multiple(vs) => {
                            write!(f, "{join}{key}={}", vs.join(", "))
                        },
                        // Only a single level of nesting makes sense.
                        LifecycleHook::Named(_) => Err(fmt::Error),
                    }
                })
            },
        }
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

impl Display for BooleanDefaultType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanDefaultType::String(value) => write!(f, "{value}"),
            BooleanDefaultType::Boolean(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum StringDevOption {
    EnumValues {
        default: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        r#enum: Vec<String>,
    },
    Proposals {
        default: Option<String>, // this field is actually required, but there are violations out there that need to be loaded.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        proposals: Option<Vec<String>>,
    },
}

impl Display for StringDevOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringDevOption::EnumValues {
                default,
                description,
                r#enum,
            } => {
                write!(
                    f,
                    "type=string, default={}, enum=[{}], description={}",
                    default,
                    r#enum.join(", "),
                    description.as_ref().cloned().unwrap_or_default(),
                )
            },
            StringDevOption::Proposals {
                default,
                description,
                proposals: Some(proposals),
            } => {
                write!(
                    f,
                    "type=string, default={}, proposals=[{}], description={}",
                    default.as_ref().cloned().unwrap_or_default(),
                    proposals.join(", "),
                    description.as_ref().cloned().unwrap_or_default(),
                )
            },
            StringDevOption::Proposals {
                default,
                description,
                proposals: None,
            } => {
                write!(
                    f,
                    "type=string, default={}, description={}",
                    default.as_ref().cloned().unwrap_or_default(),
                    description.as_ref().cloned().unwrap_or_default(),
                )
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DevOption {
    Boolean {
        default: BooleanDefaultType, // this is sometimes a bool.
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
    },
    String(StringDevOption),
}

impl Display for DevOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevOption::Boolean { default, description } => {
                write!(
                    f,
                    "type=boolean, default={default}, description={}",
                    description.as_ref().cloned().unwrap_or_default()
                )
            },
            DevOption::String(option) => write!(f, "{option}"),
        }
    }
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
            DevOption::Boolean { default, .. } => match default {
                BooleanDefaultType::String(s) => s.clone(),
                BooleanDefaultType::Boolean(b) => b.to_string(),
            },
            DevOption::String(StringDevOption::Proposals { default, proposals, .. }) => {
                // Reminder that `default` is not actually optional. This is just covering mistakes from collection maintainers.
                default
                    .clone()
                    .or_else(|| proposals.as_ref().and_then(|p| p.first().cloned()))
                    .unwrap_or_default()
            },
            DevOption::String(StringDevOption::EnumValues { default, .. }) => default.clone(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct Customizations(serde_json::Value);

impl Customizations {
    fn vscode_extensions_value(&self) -> Option<&Vec<JsonValue>> {
        self.0
            .as_object()
            .and_then(|customizations| customizations.get("vscode"))
            .and_then(|vscode_value| vscode_value.as_object())
            .and_then(|vscode| vscode.get("extensions"))
            .and_then(|extensions_value| extensions_value.as_array())
    }

    pub fn vscode_extensions(&self) -> Option<Vec<String>> {
        self.vscode_extensions_value().map(|extensions| {
            extensions
                .iter()
                .filter_map(|value| value.as_str().map(|extension_id| extension_id.to_string()))
                .collect()
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Feature {
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "documentationURL", skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(rename = "licenseURL", skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, DevOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_env: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub privileged: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_add: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_opt: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customizations: Option<Customizations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installs_after: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lecagy_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mounts: Option<Vec<DockerMount>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_create_command: Option<LifecycleHook>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_content_command: Option<LifecycleHook>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_create_command: Option<LifecycleHook>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post_start_command: Option<LifecycleHook>,
    #[serde(skip_serializing_if = "Option::is_none")]
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

impl Display for TemplateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TemplateType::Image => write!(f, "image"),
            TemplateType::Dockerfile => write!(f, "dockerfile"),
            TemplateType::DockerCompose => write!(f, "docker-compose"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub id: String,
    pub version: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "documentationURL", skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,
    #[serde(rename = "licenseURL", skip_serializing_if = "Option::is_none")]
    pub license_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, DevOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub platforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<TemplateType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_count: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
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
    collections: Vec<Collection>,
}

impl DevcontainerIndex {
    pub fn collections(&self) -> &[Collection] {
        self.collections.as_slice()
    }

    pub fn get_collection(&self, oci_reference: &str) -> Option<&Collection> {
        self.collections
            .iter()
            .find(|&collection| collection.source_information.oci_reference == oci_reference)
    }

    pub fn iter_features(&self, include_deprecated: bool) -> impl Iterator<Item = &Feature> {
        let all = |_: &&Feature| true;
        let not_deprecated = |&feature: &&Feature| feature.deprecated.map_or(true, Not::not);

        self.collections
            .iter()
            .flat_map(|collection| collection.features.iter())
            .filter(if include_deprecated { all } else { not_deprecated })
    }

    pub fn get_feature(&self, feature_id: &str) -> Option<&Feature> {
        self.iter_features(true).find(|&feature| feature.id == feature_id)
    }

    pub fn iter_templates(&self, include_deprecated: bool) -> impl Iterator<Item = &Template> {
        let all = |_: &&Collection| true;
        // There is one known collection that is deprecated, which is marked in the "maintainer" field.
        let not_deprecated = |&collection: &&Collection| {
            !collection
                .source_information
                .maintainer
                .to_lowercase()
                .contains("deprecated")
        };

        self.collections
            .iter()
            .filter(if include_deprecated { all } else { not_deprecated })
            .flat_map(|collection| collection.templates.iter())
    }

    pub fn get_template(&self, template_id: &str) -> Option<&Template> {
        self.iter_templates(true).find(|&template| template.id == template_id)
    }
}

/// Pull OCI Artifact "ghcr.io/devcontainers/index:latest" and download the JSON layer to the given filename.
pub fn pull_devcontainer_index<P: AsRef<Path>>(filename: P) -> Result<()> {
    log::debug!("pull_devcontainer_index");

    let image_name = ImageName::parse("ghcr.io/devcontainers/index:latest")?;
    let blob = get_layer_bytes(&image_name, |media_type| match media_type {
        MediaType::Other(other_type) => other_type == "application/vnd.devcontainers.index.layer.v1+json",
        _ => false,
    })
    .context("Failed to pull devcontainer index")?;
    let mut file = File::create(filename)?;

    file.write_all(&blob[..])?;

    log::debug!("pull_devcontainer_index: wrote {} bytes", blob.len());

    Ok(())
}

/// Pull bytes of the given OCI artifact, which is a reference to a given Feature or Template tar archive.
pub fn pull_archive_bytes(oci_ref: &OciReference) -> Result<Vec<u8>> {
    log::debug!("pull_archive_bytes");

    let OciReference(image_name) = oci_ref;
    let blob = get_layer_bytes(image_name, |media_type| match media_type {
        MediaType::Other(other_type) => other_type == "application/vnd.devcontainers.layer.v1+tar",
        _ => false,
    })
    .context("Failed to pull archive bytes")?;

    log::debug!("pull_archive_bytes: Pulled {} bytes for {}", blob.len(), &image_name);

    Ok(blob)
}

fn get_layer_bytes(image_name: &ImageName, f: impl Fn(&MediaType) -> bool) -> Result<Vec<u8>> {
    let mut artifact = Artifact::from_remote(image_name.clone())?;
    let manifest = artifact.get_manifest()?;
    let layer = manifest
        .layers()
        .iter()
        .find(|&d| f(d.media_type()))
        .ok_or_else(|| anyhow::anyhow!("Missing Layer"))?;
    let digest = Digest::new(layer.digest())?;

    artifact.get_blob(&digest)
}

/// Read and parse the given filename.
pub fn read_devcontainer_index<P: AsRef<Path>>(filename: P) -> Result<DevcontainerIndex, Error> {
    log::debug!("read_devcontainer_index");

    let file = fs::read_to_string(filename)?;
    let json_value: JsonValue = serde_json::from_str(&file)?;
    let mut features_count = 0;
    let mut templates_count = 0;
    let collections: Vec<Collection> = json_value
        .as_object()
        .and_then(|obj_map| obj_map.get("collections"))
        .and_then(|collections_value| collections_value.as_array())
        .map_or_else(
            || Err(Error::new(ErrorKind::InvalidData, "Unexpected json shape")),
            |arr| {
                let parsed = arr
                    .iter()
                    .filter_map(|value| {
                        let source_information: SourceInformation = match value
                            .get("sourceInformation")
                            .and_then(|value| serde_json::from_value(value.to_owned()).ok())
                        {
                            Some(value) => Some(value),
                            None => {
                                log::warn!("Skipping collection due to parsing error of sourceInformation");
                                None
                            },
                        }?;
                        let features = match value.get("features").and_then(|value| value.as_array()) {
                            Some(arr) => Some(arr),
                            None => {
                                log::warn!(
                                    "Skipping collection due to parse error. The `features` field is not an array. Collection.oci_ref = {}",
                                    &source_information.oci_reference
                                );
                                None
                            },
                        }?;
                        let features = features
                            .iter()
                            .flat_map(|value| match serde_json::from_value::<Feature>(value.to_owned()) {
                                Ok(feature) => {
                                    features_count += 1;
                                    Some(feature)
                                },
                                Err(_) => {
                                    log::warn!(
                                        "Skipping feature due to parsing error. Collection.oci_ref = {}",
                                        &source_information.oci_reference
                                    );
                                    None
                                },
                            })
                            .collect();
                        let templates = match value.get("templates").and_then(|value| value.as_array()) {
                            Some(arr) => Some(arr),
                            None => {
                                log::warn!(
                                    "Skipping collection due to parsing error. The `templates` field is not an array. Collection.oci_ref = {}",
                                    &source_information.oci_reference,
                                );
                                None
                            },
                        }?;
                        let templates = templates
                            .iter()
                            .flat_map(|value| match serde_json::from_value::<Template>(value.to_owned()) {
                                Ok(template) => {
                                    templates_count += 1;
                                    Some(template)
                                },
                                Err(_) => {
                                    log::warn!(
                                        "Skipping template due to parsing error. Collection.oci_ref = {}",
                                        &source_information.oci_reference
                                    );
                                    None
                                },
                            })
                            .collect();

                        Some(Collection {
                            source_information,
                            features,
                            templates,
                        })
                    })
                    .collect();

                Ok(parsed)
            },
        )?;

    log::debug!(
        "read_devcontainer_index: Loaded {} collections, {} features, {} templates",
        collections.len(),
        features_count,
        templates_count
    );

    Ok(DevcontainerIndex { collections })
}

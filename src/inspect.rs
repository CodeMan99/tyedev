use std::fmt::Display;
use std::error::Error;
use std::io::{self, Read, Write};

use ascii_table::{Align, AsciiTable};
use clap::{Args, ValueEnum};
use human_format::Formatter;
use tar::Archive;

use crate::registry;

#[derive(Clone, Debug, Default, ValueEnum)]
pub enum InspectDisplay {
    #[default]
    Table,
    Json,
    None,
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

    /// Use this tag when pulling from the registry.
    #[arg(short, long, default_value = "latest")]
    tag_name: String,

    /// Format for displaying the configuration.
    #[arg(short, long, value_name = "FORMAT", default_value = "table")]
    display_as: InspectDisplay,

    /// Read the `install.sh` script of a given feature.
    #[arg(long)]
    install_sh: bool,

    /// List the filenames of a given feature or template.
    #[arg(long)]
    show_files: bool,
}

struct TableData(Vec<[String; 2]>);

impl TableData {
    fn new() -> TableData {
        TableData(Vec::new())
    }

    fn push<L: Display, V: Display>(&mut self, label: L, value: V) {
        self.0.push([label.to_string(), value.to_string()]);
    }

    fn maybe_push<L: Display, V: Display>(&mut self, label: L, value: Option<V>) {
        if let Some(v) = value {
            self.0.push([label.to_string(), v.to_string()]);
        }
    }

    fn many_push<L: Display, V: Display>(&mut self, label: L, values: Option<impl IntoIterator<Item = V>>) {
        if let Some(iterable) = values {
            let mut i = iterable.into_iter();

            if let Some(first) = i.next() {
                self.0.push([label.to_string(), first.to_string()]);
            }

            for v in i {
                self.0.push([String::new(), v.to_string()]);
            }
        }
    }
}

trait Displayable: serde::Serialize {
    fn display_json(&self) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(self)?;
        println!("{json}");
        Ok(())
    }

    fn display_table(&self);
}

impl Displayable for registry::Collection {
    fn display_table(&self) {
        let mut table = AsciiTable::default();
        let data = [
            [         "Name", self.source_information.name.as_ref()],
            [   "Maintainer", self.source_information.maintainer.as_ref()],
            [      "Contact", self.source_information.contact.as_ref()],
            [   "Repository", self.source_information.repository.as_ref()],
            ["OCI Reference", self.source_information.oci_reference.as_ref()],
        ];

        table.column(0).set_align(Align::Right);
        table.print(data);
    }
}

impl Displayable for registry::Feature {
    fn display_table(&self) {
        let mut table = AsciiTable::default();
        let mut data = TableData::new();
        let comma_join = |value: &Vec<String>| value.join(", ");

        data.push("Name", self.name.clone());
        data.push("ID", self.id.clone());
        data.push("Version", self.version.clone());
        data.maybe_push("Description", self.description.as_ref());
        data.maybe_push("Documentation URL", self.documentation_url.as_ref());
        data.maybe_push("License URL", self.license_url.as_ref());
        data.maybe_push("Keywords", self.keywords.as_ref().map(comma_join));
        data.many_push("Options", self.options.as_ref().map(|options| {
            options.iter()
            .map(|(key, value)| format!("name={key}, {value}"))
            .collect::<Vec<String>>()
        }));
        data.many_push("Container ENV", self.container_env.as_ref().map(|container_env| {
            container_env.iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<String>>()
        }));
        data.maybe_push("Privileged", self.privileged);
        data.maybe_push("Init", self.init);
        data.maybe_push("Cap Add", self.cap_add.as_ref().map(comma_join));
        data.maybe_push("Security Opt", self.security_opt.as_ref().map(comma_join));
        data.maybe_push("Entrypoint", self.entrypoint.as_ref());

        let vscode_extensions =
            self.customizations.as_ref()
            .and_then(|customizations| customizations.vscode_extensions())
            .as_ref()
            .map(comma_join);

        data.maybe_push("VS Code Extensions", vscode_extensions);
        data.maybe_push("Installs After", self.installs_after.as_ref().map(comma_join));
        data.maybe_push("Legacy IDs", self.lecagy_ids.as_ref().map(comma_join));
        data.maybe_push("Deprecated", self.deprecated);
        data.many_push("Mounts", self.mounts.as_ref());
        data.maybe_push("On Create Command", self.on_create_command.as_ref());
        data.maybe_push("Update Content Command", self.update_content_command.as_ref());
        data.maybe_push("Post Create Command", self.post_create_command.as_ref());
        data.maybe_push("Post Start Command", self.post_start_command.as_ref());
        data.maybe_push("Post Attach Command", self.post_attach_command.as_ref());
        data.push("Owner", self.owner.clone());
        data.push("Major Version", self.major_version.clone());

        let TableData(inner) = data;
        table.column(0).set_align(Align::Right);
        table.print(inner);
    }
}

impl Displayable for registry::Template {
    fn display_table(&self) {
        let mut table = AsciiTable::default();
        let mut data = TableData::new();
        let comma_join = |value: &Vec<String>| value.join(", ");

        data.push("Name", self.name.clone());
        data.push("ID", self.id.clone());
        data.push("Version", self.version.clone());
        data.maybe_push("Description", self.description.as_ref());
        data.maybe_push("Documentation URL", self.documentation_url.as_ref());
        data.maybe_push("License URL", self.license_url.as_ref());
        data.many_push("Options", self.options.as_ref().map(|options| {
            options.iter()
            .map(|(key, value)| format!("name={key}, {value}"))
            .collect::<Vec<String>>()
        }));
        data.maybe_push("Platforms", self.platforms.as_ref().map(comma_join));
        data.maybe_push("Publisher", self.publisher.as_ref());
        data.maybe_push("Keywords", self.keywords.as_ref().map(comma_join));
        data.maybe_push("Type", self.r#type.as_ref());
        data.maybe_push("File Count", self.file_count);
        data.maybe_push("Feature IDs", self.feature_ids.as_ref().map(comma_join));
        data.push("Owner", self.owner.clone());

        let TableData(inner) = data;
        table.column(0).set_align(Align::Right);
        table.print(inner);
    }
}

fn display<T: ?Sized + Displayable>(value: &T, format: &InspectDisplay) -> Result<(), Box<dyn Error>> {
    match format {
        InspectDisplay::Json => value.display_json()?,
        InspectDisplay::Table => value.display_table(),
        InspectDisplay::None => println!(),
    }

    Ok(())
}

fn display_files(id: &str, tag_name: &str) -> Result<(), Box<dyn Error>> {
    let bytes = registry::pull_archive_bytes(id, tag_name)?;
    let mut archive = Archive::new(bytes.as_slice());
    let entries = archive.entries()?;

    for entry in entries {
        let entry = entry?;
        let header = entry.header();
        let size = header.size()?;

        if size > 0 {
            let human_size = Formatter::new().with_decimals(1).format(size as f64);
            let filename = header.path()?;

            // Example max expected string length: "123.4 k" - which is seven characters.
            println!("{:>width$}: {}", human_size.trim_end(), filename.display(), width = 7);
        }
    }

    Ok(())
}

fn display_install_sh(id: &str, tag_name: &str) -> Result<(), Box<dyn Error>> {
    let bytes = registry::pull_archive_bytes(id, tag_name)?;
    let mut archive = Archive::new(bytes.as_slice());
    let entries = archive.entries()?;

    for entry in entries {
        let mut entry = entry?;
        let path = entry.path()?;

        if path.to_str() == Some("./install.sh") {
            let mut data: Vec<u8> = Vec::new();

            entry.read_to_end(&mut data)?;
            io::stdout().write_all(data.as_slice())?;

            return Ok(());
        }
    }

    Err(io::Error::new(io::ErrorKind::NotFound, "The install.sh script was not found in the archive"))?
}

pub fn inspect(
    index: &registry::DevcontainerIndex,
    InspectArgs {
        id,
        tag_name,
        display_as,
        install_sh,
        show_files,
    }: InspectArgs
) -> Result<(), Box<dyn Error>> {
    let collection = index.get_collection(&id);
    let feature = index.get_feature(&id);
    let template = index.get_template(&id);

    match (collection, feature, template) {
        (Some(c), None, None) => {
            display(c, &display_as)?;

            if show_files || install_sh {
                eprintln!("WARNING: A collection is container of features & templates, not files.");
            }

            Ok(())
        },
        (None, Some(f), None) => {
            display(f, &display_as)?;

            if show_files {
                display_files(&id, &tag_name)?;
            }

            if install_sh {
                display_install_sh(&id, &tag_name)?;
            }

            Ok(())
        },
        (None, None, Some(t)) => {
            display(t, &display_as)?;

            if show_files {
                display_files(&id, &tag_name)?;
            }

            if install_sh {
                eprintln!("WARNING: Templates are not required to have an install.sh file.");
            }

            Ok(())
        },
        (None, None, None) => Err(io::Error::new(io::ErrorKind::NotFound, "No match found for given id.")),
        _ => Err(io::Error::new(io::ErrorKind::Unsupported, "Multiple results found for given id.")),
    }?;

    Ok(())
}

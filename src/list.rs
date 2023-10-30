use ascii_table::{Align, AsciiTable};

use crate::registry::{Collection, DevcontainerIndex};
use crate::search;

pub fn collection_templates_and_features(oci_reference: &str, collection: &Collection) -> () {
    let source_information = &collection.source_information;

    println!("Name:          {}", &source_information.name);
    println!("Maintainer:    {}", &source_information.maintainer);
    println!("Contact:       {}", &source_information.contact);
    println!("Repository:    {}", &source_information.repository);
    println!("OCI Reference: {}", &source_information.oci_reference);

    let search_results = {
        let features = collection.features.iter().map(search::SearchResult::from);
        let templates = collection.templates.iter().map(search::SearchResult::from);
        features.chain(templates)
    };
    let data: Vec<[String; 5]> =
        search_results.enumerate().map(|(i, r)| {
            let description =
                r.description.as_ref()
                .and_then(|d| d.lines().next())
                .unwrap_or_default();
            [
                format!("{}", i + 1),
                format!("{}", r.collection),
                format!("{}", r.id.replace(oci_reference, "~")),
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
}

pub fn overview_collections(index: &DevcontainerIndex) -> () {
    let mut table = AsciiTable::default();

    table.column(0).set_header("Name");
    table.column(1).set_header("OCI Reference");
    table.column(2).set_header("Features").set_align(Align::Right);
    table.column(3).set_header("Templates").set_align(Align::Right);

    let result: Vec<[String; 4]> =
        index.collections
        .iter()
        .map(|collection| [
            format!("{}", collection.source_information.name),
            format!("{}", collection.source_information.oci_reference),
            format!("{}", collection.features.len()),
            format!("{}", collection.templates.len()),
        ])
        .collect();

    table.print(result);
}
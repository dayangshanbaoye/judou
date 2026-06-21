use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    path::Path,
};

use quick_xml::{events::Event, Reader};
use serde::Serialize;
use zip::ZipArchive;

use crate::error::{JudouError, Result};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EpubMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpineItem {
    pub idref: String,
    pub href: String,
    pub media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageDocument {
    pub opf_path: String,
    pub metadata: EpubMetadata,
    pub spine: Vec<SpineItem>,
    pub ncx_href: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ManifestItem {
    href: String,
    media_type: String,
}

pub fn read_package_document(path: &Path) -> Result<PackageDocument> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let container_xml = read_archive_text(&mut archive, "META-INF/container.xml")?;
    let opf_path = locate_opf_path(&container_xml)?;
    let opf_xml = read_archive_text(&mut archive, &opf_path)?;
    parse_package_document(&opf_path, &opf_xml)
}

pub fn locate_opf_path(container_xml: &str) -> Result<String> {
    let mut reader = Reader::from_str(container_xml);
    reader.config_mut().trim_text(true);

    loop {
        match reader.read_event()? {
            Event::Empty(element) | Event::Start(element)
                if local_name(element.name().as_ref()) == b"rootfile" =>
            {
                if let Some(path) = attribute_value(&reader, &element, b"full-path")? {
                    return Ok(path);
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Err(JudouError::Validation(
        "EPUB container.xml missing rootfile full-path".to_string(),
    ))
}

pub fn parse_package_document(opf_path: &str, opf_xml: &str) -> Result<PackageDocument> {
    let mut reader = Reader::from_str(opf_xml);
    reader.config_mut().trim_text(true);

    let mut metadata = EpubMetadata {
        title: None,
        author: None,
        language: None,
    };
    let mut manifest = HashMap::new();
    let mut spine_idrefs = Vec::new();
    let mut toc_id = None;
    let mut current_text_field = None;

    loop {
        match reader.read_event()? {
            Event::Start(element) => match local_name(element.name().as_ref()) {
                b"title" => current_text_field = Some(MetadataField::Title),
                b"creator" => current_text_field = Some(MetadataField::Author),
                b"language" => current_text_field = Some(MetadataField::Language),
                b"spine" => {
                    toc_id = attribute_value(&reader, &element, b"toc")?;
                }
                _ => {}
            },
            Event::Empty(element) => match local_name(element.name().as_ref()) {
                b"item" => {
                    let id = required_attribute(&reader, &element, b"id")?;
                    let href = required_attribute(&reader, &element, b"href")?;
                    let media_type = required_attribute(&reader, &element, b"media-type")?;
                    manifest.insert(
                        id,
                        ManifestItem {
                            href: resolve_href(opf_path, &href),
                            media_type,
                        },
                    );
                }
                b"itemref" => {
                    let idref = required_attribute(&reader, &element, b"idref")?;
                    spine_idrefs.push(idref);
                }
                _ => {}
            },
            Event::Text(text) => {
                if let Some(field) = current_text_field.take() {
                    let value = text
                        .xml_content()
                        .map_err(|error| JudouError::Validation(error.to_string()))?
                        .trim()
                        .to_string();
                    if !value.is_empty() {
                        match field {
                            MetadataField::Title => metadata.title = Some(value),
                            MetadataField::Author => metadata.author = Some(value),
                            MetadataField::Language => metadata.language = Some(value),
                        }
                    }
                }
            }
            Event::End(_) => current_text_field = None,
            Event::Eof => break,
            _ => {}
        }
    }

    let mut spine = Vec::with_capacity(spine_idrefs.len());
    for idref in spine_idrefs {
        let item = manifest.get(&idref).ok_or_else(|| {
            JudouError::Validation(format!("spine references missing manifest item '{idref}'"))
        })?;
        spine.push(SpineItem {
            idref,
            href: item.href.clone(),
            media_type: item.media_type.clone(),
        });
    }

    let ncx_href = toc_id
        .and_then(|id| manifest.get(&id).map(|item| item.href.clone()))
        .or_else(|| {
            manifest
                .values()
                .find(|item| item.media_type == "application/x-dtbncx+xml")
                .map(|item| item.href.clone())
        });

    Ok(PackageDocument {
        opf_path: opf_path.to_string(),
        metadata,
        spine,
        ncx_href,
    })
}

fn read_archive_text(archive: &mut ZipArchive<File>, path: &str) -> Result<String> {
    let mut file = archive.by_name(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn required_attribute(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    key: &[u8],
) -> Result<String> {
    attribute_value(reader, element, key)?.ok_or_else(|| {
        JudouError::Validation(format!(
            "missing required XML attribute '{}'",
            String::from_utf8_lossy(key)
        ))
    })
}

fn attribute_value(
    reader: &Reader<&[u8]>,
    element: &quick_xml::events::BytesStart<'_>,
    key: &[u8],
) -> Result<Option<String>> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| JudouError::Validation(error.to_string()))?;
        if local_name(attribute.key.as_ref()) == key {
            let value = attribute.decode_and_unescape_value(reader.decoder())?;
            return Ok(Some(value.into_owned()));
        }
    }
    Ok(None)
}

fn local_name(name: &[u8]) -> &[u8] {
    match name.rsplit(|byte| *byte == b':').next() {
        Some(local) => local,
        None => name,
    }
}

fn resolve_href(opf_path: &str, href: &str) -> String {
    if href.starts_with('/') {
        return href.trim_start_matches('/').to_string();
    }

    match opf_path.rsplit_once('/') {
        Some((base, _)) if !base.is_empty() => format!("{base}/{href}"),
        _ => href.to_string(),
    }
}

#[derive(Debug, Clone, Copy)]
enum MetadataField {
    Title,
    Author,
    Language,
}

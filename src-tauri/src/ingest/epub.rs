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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Introduction,
    Preface,
    Body,
    TitleOnly,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TocNode {
    pub title: String,
    pub href: String,
    pub anchor: Option<String>,
    pub level: usize,
    pub order_index: usize,
    pub content_type: ContentType,
    pub included: bool,
    pub children: Vec<TocNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtractedParagraph {
    pub order_index: usize,
    pub source_href: String,
    pub source_path: String,
    pub clean_text: String,
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

pub fn parse_ncx_toc(path: &Path, ncx_href: &str) -> Result<Vec<TocNode>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let ncx_xml = read_archive_text(&mut archive, ncx_href)?;
    parse_ncx_document(&ncx_xml)
}

pub fn parse_ncx_document(ncx_xml: &str) -> Result<Vec<TocNode>> {
    let mut reader = Reader::from_str(ncx_xml);
    reader.config_mut().trim_text(true);

    let mut roots = Vec::new();
    let mut stack: Vec<TocNode> = Vec::new();
    let mut in_nav_label = false;
    let mut in_label_text = false;

    loop {
        match reader.read_event()? {
            Event::Start(element) => match local_name(element.name().as_ref()) {
                b"navPoint" => {
                    let level = stack.len() + 1;
                    let order_index = match stack.last() {
                        Some(parent) => parent.children.len(),
                        None => roots.len(),
                    };
                    stack.push(TocNode {
                        title: String::new(),
                        href: String::new(),
                        anchor: None,
                        level,
                        order_index,
                        content_type: ContentType::Body,
                        included: true,
                        children: Vec::new(),
                    });
                }
                b"navLabel" if !stack.is_empty() => in_nav_label = true,
                b"text" if in_nav_label => in_label_text = true,
                _ => {}
            },
            Event::Empty(element) if local_name(element.name().as_ref()) == b"content" => {
                if let Some(node) = stack.last_mut() {
                    if let Some(src) = attribute_value(&reader, &element, b"src")? {
                        let (href, anchor) = split_href_anchor(&src);
                        node.href = href;
                        node.anchor = anchor;
                    }
                }
            }
            Event::Text(text) if in_label_text => {
                if let Some(node) = stack.last_mut() {
                    node.title = text
                        .xml_content()
                        .map_err(|error| JudouError::Validation(error.to_string()))?
                        .trim()
                        .to_string();
                }
            }
            Event::End(element) => match local_name(element.name().as_ref()) {
                b"navPoint" => {
                    let node = stack.pop().ok_or_else(|| {
                        JudouError::Validation("unbalanced NCX navPoint".to_string())
                    })?;
                    if let Some(parent) = stack.last_mut() {
                        parent.children.push(node);
                    } else {
                        roots.push(node);
                    }
                }
                b"navLabel" => {
                    in_nav_label = false;
                    in_label_text = false;
                }
                b"text" => in_label_text = false,
                _ => {}
            },
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(roots)
}

pub fn classify_toc_nodes(nodes: &mut [TocNode]) {
    for node in nodes {
        classify_toc_node(node);
    }
}

pub fn extract_paragraphs_from_xhtml(path: &Path, href: &str) -> Result<Vec<ExtractedParagraph>> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let xhtml = read_archive_text(&mut archive, href)?;
    extract_paragraphs_from_xhtml_document(href, &xhtml)
}

pub fn extract_paragraphs_from_xhtml_document(
    href: &str,
    xhtml: &str,
) -> Result<Vec<ExtractedParagraph>> {
    let mut reader = Reader::from_str(xhtml);
    reader.config_mut().trim_text(false);

    let mut paragraphs = Vec::new();
    let mut paragraph_index = 0usize;
    let mut paragraph_depth = 0usize;
    let mut current_path = String::new();
    let mut current_text = String::new();

    loop {
        match reader.read_event()? {
            Event::Start(element) if local_name(element.name().as_ref()) == b"p" => {
                if paragraph_depth == 0 {
                    paragraph_index += 1;
                    current_path = format!("p[{paragraph_index}]");
                    current_text.clear();
                }
                paragraph_depth += 1;
            }
            Event::Start(_) if paragraph_depth > 0 => {
                paragraph_depth += 1;
            }
            Event::Text(text) if paragraph_depth > 0 => {
                let value = text
                    .xml_content()
                    .map_err(|error| JudouError::Validation(error.to_string()))?;
                current_text.push_str(&value);
            }
            Event::CData(text) if paragraph_depth > 0 => {
                let value = text
                    .decode()
                    .map_err(|error| JudouError::Validation(error.to_string()))?;
                current_text.push_str(&value);
            }
            Event::End(element) if paragraph_depth > 0 => {
                let ending_paragraph =
                    local_name(element.name().as_ref()) == b"p" && paragraph_depth == 1;
                paragraph_depth -= 1;

                if ending_paragraph {
                    let clean_text = normalize_text(&current_text);
                    if is_reading_paragraph(&clean_text) {
                        paragraphs.push(ExtractedParagraph {
                            order_index: paragraphs.len(),
                            source_href: href.to_string(),
                            source_path: current_path.clone(),
                            clean_text,
                        });
                    }
                    current_text.clear();
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(paragraphs)
}

fn classify_toc_node(node: &mut TocNode) {
    for child in &mut node.children {
        classify_toc_node(child);
    }

    node.content_type = classify_title(&node.title, !node.children.is_empty());
    node.included = matches!(
        node.content_type,
        ContentType::Introduction | ContentType::Preface | ContentType::Body
    );
}

fn classify_title(title: &str, has_children: bool) -> ContentType {
    let normalized = title.trim().to_ascii_lowercase();

    if has_children || normalized.starts_with("part ") {
        return ContentType::TitleOnly;
    }

    if normalized.starts_with("introduction") {
        return ContentType::Introduction;
    }

    if normalized.contains("preface") || normalized.contains("foreword") {
        return ContentType::Preface;
    }

    let excluded_titles = [
        "cover",
        "also by",
        "title page",
        "copyright",
        "contents",
        "dedication",
        "epigraph",
        "acknowledgments",
        "notes",
        "index",
        "about the author",
    ];
    if excluded_titles
        .iter()
        .any(|excluded| normalized == *excluded || normalized.starts_with(excluded))
    {
        return ContentType::Excluded;
    }

    ContentType::Body
}

fn is_reading_paragraph(text: &str) -> bool {
    text.chars().count() >= 30 && text.chars().any(|ch| matches!(ch, '.' | '?' | '!' | '…'))
}

fn normalize_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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

fn split_href_anchor(src: &str) -> (String, Option<String>) {
    match src.split_once('#') {
        Some((href, anchor)) => (href.to_string(), Some(anchor.to_string())),
        None => (src.to_string(), None),
    }
}

#[derive(Debug, Clone, Copy)]
enum MetadataField {
    Title,
    Author,
    Language,
}

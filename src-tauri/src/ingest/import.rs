use std::{
    fs::File,
    io::Read,
    path::Path,
};

use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    error::{JudouError, Result},
    ingest::epub::{
        classify_toc_nodes, extract_paragraphs_from_xhtml, parse_ncx_toc, read_package_document,
        ContentType, TocNode,
    },
    repo::{BookDraft, ChapterParagraphs, SqliteRepo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportOptions {
    pub max_included_chapters: Option<usize>,
}

impl ImportOptions {
    pub fn full() -> Self {
        Self {
            max_included_chapters: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportedBook {
    pub book_id: i64,
    pub report: ImportReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub book_id: i64,
    pub title: String,
    pub root_toc_nodes: usize,
    pub toc_nodes_total: usize,
    pub included_toc_nodes: usize,
    pub title_only_toc_nodes: usize,
    pub excluded_toc_nodes: usize,
    pub chapters_imported: usize,
    pub paragraphs_imported: usize,
}

pub fn import_epub(
    connection: &Connection,
    path: &Path,
    options: ImportOptions,
) -> Result<ImportedBook> {
    let package = read_package_document(path)?;
    let ncx_href = package
        .ncx_href
        .as_deref()
        .ok_or_else(|| JudouError::Validation("EPUB package missing NCX href".to_string()))?;
    let mut toc_nodes = parse_ncx_toc(path, ncx_href)?;
    classify_toc_nodes(&mut toc_nodes);

    let included_hrefs = included_hrefs(&toc_nodes, options.max_included_chapters);
    let mut chapter_paragraphs = Vec::with_capacity(included_hrefs.len());
    let mut paragraphs_imported = 0usize;
    for href in &included_hrefs {
        let paragraphs = extract_paragraphs_from_xhtml(path, href)?;
        paragraphs_imported += paragraphs.len();
        chapter_paragraphs.push(ChapterParagraphs {
            toc_href: href.as_str(),
            paragraphs,
        });
    }

    let stats = TocStats::from_nodes(&toc_nodes);
    let title = match package.metadata.title.as_deref() {
        Some(title) => title.to_string(),
        None => "Untitled".to_string(),
    };
    let file_hash = file_sha256(path)?;
    let repo = SqliteRepo::new(connection);
    let book_id = repo.insert_book_structure(
        &BookDraft {
            title: &title,
            author: package.metadata.author.as_deref(),
            language: package.metadata.language.as_deref(),
            file_hash: &file_hash,
        },
        &toc_nodes,
        &chapter_paragraphs,
    )?;

    Ok(ImportedBook {
        book_id,
        report: ImportReport {
            book_id,
            title,
            root_toc_nodes: toc_nodes.len(),
            toc_nodes_total: stats.total,
            included_toc_nodes: stats.included,
            title_only_toc_nodes: stats.title_only,
            excluded_toc_nodes: stats.excluded,
            chapters_imported: included_hrefs.len(),
            paragraphs_imported,
        },
    })
}

fn included_hrefs(nodes: &[TocNode], max_included_chapters: Option<usize>) -> Vec<String> {
    let mut hrefs = Vec::new();
    collect_included_hrefs(nodes, max_included_chapters, &mut hrefs);
    hrefs
}

fn collect_included_hrefs(
    nodes: &[TocNode],
    max_included_chapters: Option<usize>,
    hrefs: &mut Vec<String>,
) {
    for node in nodes {
        if max_included_chapters.is_some_and(|max| hrefs.len() >= max) {
            return;
        }

        if node.included && !node.href.is_empty() {
            hrefs.push(node.href.clone());
        }

        collect_included_hrefs(&node.children, max_included_chapters, hrefs);
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TocStats {
    total: usize,
    included: usize,
    title_only: usize,
    excluded: usize,
}

impl TocStats {
    fn from_nodes(nodes: &[TocNode]) -> Self {
        let mut stats = Self::default();
        stats.add_nodes(nodes);
        stats
    }

    fn add_nodes(&mut self, nodes: &[TocNode]) {
        for node in nodes {
            self.total += 1;
            if node.included {
                self.included += 1;
            }
            match node.content_type {
                ContentType::TitleOnly => self.title_only += 1,
                ContentType::Excluded => self.excluded += 1,
                ContentType::Introduction | ContentType::Preface | ContentType::Body => {}
            }
            self.add_nodes(&node.children);
        }
    }
}

fn file_sha256(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }

    Ok(format!("{:x}", hasher.finalize()))
}

use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    domain::ImportReport,
    error::{JudouError, Result},
    ingest::epub::{ContentType, ExtractedParagraph, TocNode},
};

pub struct SqliteRepo<'connection> {
    connection: &'connection Connection,
}

impl<'connection> SqliteRepo<'connection> {
    pub fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub fn insert_book_structure(
        &self,
        book: &BookDraft<'_>,
        toc_nodes: &[TocNode],
        chapter_paragraphs: &[ChapterParagraphs<'_>],
    ) -> Result<i64> {
        self.connection.execute(
            "DELETE FROM books WHERE file_hash = ?1",
            params![book.file_hash],
        )?;
        self.connection.execute(
            "INSERT INTO books(title, author, language, file_hash) VALUES (?1, ?2, ?3, ?4)",
            params![book.title, book.author, book.language, book.file_hash],
        )?;
        let book_id = self.connection.last_insert_rowid();

        for node in toc_nodes {
            self.insert_toc_node(book_id, None, node)?;
        }

        for chapter in chapter_paragraphs {
            let toc_node_id = self.find_toc_node_id_by_href(book_id, chapter.toc_href)?;
            for paragraph in &chapter.paragraphs {
                self.connection.execute(
                    "INSERT INTO paragraphs(book_id, toc_node_id, order_index, source_href, source_path, clean_text)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        book_id,
                        toc_node_id,
                        paragraph.order_index as i64,
                        paragraph.source_href,
                        paragraph.source_path,
                        paragraph.clean_text
                    ],
                )?;
            }
        }

        Ok(book_id)
    }

    pub fn find_paragraph_trace(
        &self,
        book_id: i64,
        source_href: &str,
        paragraph_order_index: usize,
    ) -> Result<ParagraphTrace> {
        self.connection
            .query_row(
                "SELECT b.title, t.title, p.clean_text
                 FROM paragraphs p
                 JOIN toc_nodes t ON t.id = p.toc_node_id
                 JOIN books b ON b.id = p.book_id
                 WHERE p.book_id = ?1 AND p.source_href = ?2 AND p.order_index = ?3",
                params![book_id, source_href, paragraph_order_index as i64],
                |row| {
                    Ok(ParagraphTrace {
                        book_title: row.get(0)?,
                        toc_title: row.get(1)?,
                        clean_text: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn count_toc_nodes(&self, book_id: i64) -> Result<i64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1",
                params![book_id],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub fn get_import_report(&self, book_id: i64) -> Result<ImportReport> {
        let title = self
            .connection
            .query_row(
                "SELECT title FROM books WHERE id = ?1",
                params![book_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| JudouError::Validation(format!("book '{book_id}' not found")))?;

        let root_toc_nodes = self.count_i64(
            "SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1 AND parent_id IS NULL",
            book_id,
        )? as usize;
        let toc_nodes_total =
            self.count_i64("SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1", book_id)?
                as usize;
        let included_toc_nodes = self.count_i64(
            "SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1 AND included = 1",
            book_id,
        )? as usize;
        let title_only_toc_nodes = self.count_i64(
            "SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1 AND content_type = 'title_only'",
            book_id,
        )? as usize;
        let excluded_toc_nodes = self.count_i64(
            "SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1 AND content_type = 'excluded'",
            book_id,
        )? as usize;
        let chapters_imported = self.count_i64(
            "SELECT COUNT(DISTINCT source_href) FROM paragraphs WHERE book_id = ?1",
            book_id,
        )? as usize;
        let paragraphs_imported =
            self.count_i64("SELECT COUNT(*) FROM paragraphs WHERE book_id = ?1", book_id)? as usize;

        Ok(ImportReport {
            book_id,
            title,
            root_toc_nodes,
            toc_nodes_total,
            included_toc_nodes,
            title_only_toc_nodes,
            excluded_toc_nodes,
            chapters_imported,
            paragraphs_imported,
        })
    }

    fn insert_toc_node(&self, book_id: i64, parent_id: Option<i64>, node: &TocNode) -> Result<i64> {
        self.connection.execute(
            "INSERT INTO toc_nodes(book_id, parent_id, title, level, order_index, spine_href, nav_anchor, content_type, included)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                book_id,
                parent_id,
                node.title,
                node.level as i64,
                node.order_index as i64,
                node.href,
                node.anchor,
                content_type_value(node.content_type),
                bool_value(node.included)
            ],
        )?;
        let toc_node_id = self.connection.last_insert_rowid();

        for child in &node.children {
            self.insert_toc_node(book_id, Some(toc_node_id), child)?;
        }

        Ok(toc_node_id)
    }

    fn find_toc_node_id_by_href(&self, book_id: i64, href: &str) -> Result<i64> {
        self.connection
            .query_row(
                "SELECT id FROM toc_nodes WHERE book_id = ?1 AND spine_href = ?2 ORDER BY level DESC, order_index LIMIT 1",
                params![book_id, href],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                JudouError::Validation(format!(
                    "cannot attach paragraphs: missing toc node for href '{href}'"
                ))
            })
    }

    fn count_i64(&self, sql: &str, book_id: i64) -> Result<i64> {
        self.connection
            .query_row(sql, params![book_id], |row| row.get(0))
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BookDraft<'value> {
    pub title: &'value str,
    pub author: Option<&'value str>,
    pub language: Option<&'value str>,
    pub file_hash: &'value str,
}

#[derive(Debug, Clone)]
pub struct ChapterParagraphs<'value> {
    pub toc_href: &'value str,
    pub paragraphs: Vec<ExtractedParagraph>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphTrace {
    pub book_title: String,
    pub toc_title: String,
    pub clean_text: String,
}

fn content_type_value(content_type: ContentType) -> &'static str {
    match content_type {
        ContentType::Introduction => "introduction",
        ContentType::Preface => "preface",
        ContentType::Body => "body",
        ContentType::TitleOnly => "title_only",
        ContentType::Excluded => "excluded",
    }
}

fn bool_value(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

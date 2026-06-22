use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    domain::{
        ImportReport, ReaderBreadcrumb, ReaderParagraph, ReaderSentence, ReaderTocNode, ReaderView,
    },
    error::{JudouError, Result},
    ingest::epub::{ContentType, ExtractedParagraph, TocNode},
    segment::{SegmentationOutput, SegmentedSentence},
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

    pub fn find_paragraph(
        &self,
        book_id: i64,
        source_href: &str,
        paragraph_order_index: usize,
    ) -> Result<StoredParagraph> {
        self.connection
            .query_row(
                "SELECT id, source_href, source_path, clean_text
                 FROM paragraphs
                 WHERE book_id = ?1 AND source_href = ?2 AND order_index = ?3",
                params![book_id, source_href, paragraph_order_index as i64],
                |row| {
                    Ok(StoredParagraph {
                        id: row.get(0)?,
                        source_href: row.get(1)?,
                        source_path: row.get(2)?,
                        clean_text: row.get(3)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn insert_sentences(
        &self,
        paragraph_id: i64,
        sentences: &[SegmentedSentence],
    ) -> Result<()> {
        let (book_id, clean_text): (i64, String) = self.connection.query_row(
            "SELECT book_id, clean_text FROM paragraphs WHERE id = ?1",
            params![paragraph_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let joined = sentences
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<String>();
        if joined != clean_text {
            return Err(JudouError::Validation(
                "segmented sentences do not reconstruct source paragraph".to_string(),
            ));
        }

        self.connection.execute(
            "DELETE FROM sentences WHERE paragraph_id = ?1",
            params![paragraph_id],
        )?;

        for (order_index, sentence) in sentences.iter().enumerate() {
            self.connection.execute(
                "INSERT INTO sentences(book_id, paragraph_id, order_index, text, normalized_text, start_offset, end_offset, segmentation_method, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'rule', 'unread')",
                params![
                    book_id,
                    paragraph_id,
                    order_index as i64,
                    sentence.text,
                    sentence.text.trim(),
                    sentence.start_offset as i64,
                    sentence.end_offset as i64,
                ],
            )?;
        }

        Ok(())
    }

    pub fn segment_book_paragraphs(
        &self,
        book_id: i64,
        segmenter: fn(&str) -> SegmentationOutput,
    ) -> Result<usize> {
        let mut statement = self.connection.prepare(
            "SELECT id, source_href, source_path, clean_text FROM paragraphs WHERE book_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map(params![book_id], |row| {
            Ok(StoredParagraph {
                id: row.get(0)?,
                source_href: row.get(1)?,
                source_path: row.get(2)?,
                clean_text: row.get(3)?,
            })
        })?;

        let mut paragraphs = Vec::new();
        for row in rows {
            paragraphs.push(row?);
        }

        let mut sentence_count = 0usize;
        for paragraph in paragraphs {
            let output = segmenter(&paragraph.clean_text);
            sentence_count += output.sentences.len();
            self.insert_sentences(paragraph.id, &output.sentences)?;
            for notice in output.notices {
                self.connection.execute(
                    "INSERT INTO processing_log(book_id, stage, severity, location_ref, raw_snippet, action_taken, source, resolved)
                     VALUES (?1, 'segment', 'info', ?2, ?3, ?4, 'rule', 0)",
                    params![
                        book_id,
                        format!(
                            "{}:{}@{}",
                            paragraph.source_href, paragraph.source_path, notice.offset
                        ),
                        notice.snippet,
                        format!("{}: {}", notice.rule_name, notice.action_taken),
                    ],
                )?;
            }
        }

        Ok(sentence_count)
    }

    pub fn find_sentence_trace(
        &self,
        book_id: i64,
        source_href: &str,
        paragraph_order_index: usize,
        sentence_order_index: usize,
    ) -> Result<SentenceTrace> {
        self.connection
            .query_row(
                "SELECT b.title, t.title, p.clean_text, s.text, s.segmentation_method, s.status
                 FROM sentences s
                 JOIN paragraphs p ON p.id = s.paragraph_id
                 JOIN toc_nodes t ON t.id = p.toc_node_id
                 JOIN books b ON b.id = s.book_id
                 WHERE s.book_id = ?1
                   AND p.source_href = ?2
                   AND p.order_index = ?3
                   AND s.order_index = ?4",
                params![
                    book_id,
                    source_href,
                    paragraph_order_index as i64,
                    sentence_order_index as i64,
                ],
                |row| {
                    Ok(SentenceTrace {
                        book_title: row.get(0)?,
                        toc_title: row.get(1)?,
                        paragraph_text: row.get(2)?,
                        sentence_text: row.get(3)?,
                        segmentation_method: row.get(4)?,
                        status: row.get(5)?,
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

    pub fn count_processing_log(&self, book_id: i64, stage: &str) -> Result<i64> {
        self.connection
            .query_row(
                "SELECT COUNT(*) FROM processing_log WHERE book_id = ?1 AND stage = ?2",
                params![book_id, stage],
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
            self.count_i64("SELECT COUNT(*) FROM toc_nodes WHERE book_id = ?1", book_id)? as usize;
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
        let paragraphs_imported = self.count_i64(
            "SELECT COUNT(*) FROM paragraphs WHERE book_id = ?1",
            book_id,
        )? as usize;
        let sentences_imported =
            self.count_i64("SELECT COUNT(*) FROM sentences WHERE book_id = ?1", book_id)? as usize;

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
            sentences_imported,
        })
    }

    pub fn get_reader_view(&self, book_id: i64, toc_node_id: Option<i64>) -> Result<ReaderView> {
        let book_title = self
            .connection
            .query_row(
                "SELECT title FROM books WHERE id = ?1",
                params![book_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or_else(|| JudouError::Validation(format!("book '{book_id}' not found")))?;
        let active_toc_node_id = match toc_node_id {
            Some(id) => id,
            None => self.first_readable_toc_node_id(book_id)?,
        };

        Ok(ReaderView {
            book_id,
            book_title,
            active_toc_node_id,
            breadcrumb: self.reader_breadcrumb(active_toc_node_id)?,
            toc_nodes: self.reader_toc_nodes(book_id)?,
            paragraphs: self.reader_paragraphs(active_toc_node_id)?,
        })
    }

    pub fn update_sentence_status(&self, sentence_id: i64, status: &str) -> Result<ReaderSentence> {
        match status {
            "unread" | "read" | "understood" | "flagged" => {}
            _ => {
                return Err(JudouError::Validation(format!(
                    "invalid sentence status '{status}'"
                )));
            }
        }

        let changed = self.connection.execute(
            "UPDATE sentences SET status = ?1 WHERE id = ?2",
            params![status, sentence_id],
        )?;
        if changed == 0 {
            return Err(JudouError::Validation(format!(
                "sentence '{sentence_id}' not found"
            )));
        }

        self.find_reader_sentence(sentence_id)
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

    fn first_readable_toc_node_id(&self, book_id: i64) -> Result<i64> {
        self.connection
            .query_row(
                "SELECT t.id
                 FROM toc_nodes t
                 WHERE t.book_id = ?1
                   AND t.included = 1
                   AND EXISTS (SELECT 1 FROM paragraphs p WHERE p.toc_node_id = t.id)
                 ORDER BY t.id
                 LIMIT 1",
                params![book_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or_else(|| {
                JudouError::Validation(format!("book '{book_id}' has no readable toc node"))
            })
    }

    fn reader_toc_nodes(&self, book_id: i64) -> Result<Vec<ReaderTocNode>> {
        let mut statement = self.connection.prepare(
            "SELECT id, parent_id, title, level, order_index, content_type, included
             FROM toc_nodes
             WHERE book_id = ?1
             ORDER BY id",
        )?;
        let rows = statement.query_map(params![book_id], |row| {
            let included: i64 = row.get(6)?;
            Ok(ReaderTocNode {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                title: row.get(2)?,
                level: row.get(3)?,
                order_index: row.get(4)?,
                content_type: row.get(5)?,
                included: included != 0,
            })
        })?;

        let mut nodes = Vec::new();
        for row in rows {
            nodes.push(row?);
        }
        Ok(nodes)
    }

    fn reader_breadcrumb(&self, toc_node_id: i64) -> Result<Vec<ReaderBreadcrumb>> {
        let mut breadcrumb = Vec::new();
        let mut current_id = Some(toc_node_id);
        while let Some(id) = current_id {
            let node = self
                .connection
                .query_row(
                    "SELECT parent_id, title FROM toc_nodes WHERE id = ?1",
                    params![id],
                    |row| Ok((row.get::<_, Option<i64>>(0)?, row.get::<_, String>(1)?)),
                )
                .optional()?
                .ok_or_else(|| JudouError::Validation(format!("toc node '{id}' not found")))?;
            breadcrumb.push(ReaderBreadcrumb { id, title: node.1 });
            current_id = node.0;
        }
        breadcrumb.reverse();
        Ok(breadcrumb)
    }

    fn reader_paragraphs(&self, toc_node_id: i64) -> Result<Vec<ReaderParagraph>> {
        let mut statement = self.connection.prepare(
            "SELECT id, toc_node_id, order_index, source_href
             FROM paragraphs
             WHERE toc_node_id = ?1
             ORDER BY order_index, id",
        )?;
        let rows = statement.query_map(params![toc_node_id], |row| {
            Ok(ReaderParagraph {
                id: row.get(0)?,
                toc_node_id: row.get(1)?,
                order_index: row.get(2)?,
                source_href: row.get(3)?,
                sentences: Vec::new(),
            })
        })?;

        let mut paragraphs = Vec::new();
        for row in rows {
            let mut paragraph = row?;
            paragraph.sentences = self.reader_sentences(paragraph.id)?;
            paragraphs.push(paragraph);
        }
        Ok(paragraphs)
    }

    fn reader_sentences(&self, paragraph_id: i64) -> Result<Vec<ReaderSentence>> {
        let mut statement = self.connection.prepare(
            "SELECT id, paragraph_id, order_index, text, status
             FROM sentences
             WHERE paragraph_id = ?1
             ORDER BY order_index, id",
        )?;
        let rows = statement.query_map(params![paragraph_id], |row| {
            Ok(ReaderSentence {
                id: row.get(0)?,
                paragraph_id: row.get(1)?,
                order_index: row.get(2)?,
                text: row.get(3)?,
                status: row.get(4)?,
            })
        })?;

        let mut sentences = Vec::new();
        for row in rows {
            sentences.push(row?);
        }
        Ok(sentences)
    }

    fn find_reader_sentence(&self, sentence_id: i64) -> Result<ReaderSentence> {
        self.connection
            .query_row(
                "SELECT id, paragraph_id, order_index, text, status
                 FROM sentences
                 WHERE id = ?1",
                params![sentence_id],
                |row| {
                    Ok(ReaderSentence {
                        id: row.get(0)?,
                        paragraph_id: row.get(1)?,
                        order_index: row.get(2)?,
                        text: row.get(3)?,
                        status: row.get(4)?,
                    })
                },
            )
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredParagraph {
    pub id: i64,
    pub source_href: String,
    pub source_path: String,
    pub clean_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SentenceTrace {
    pub book_title: String,
    pub toc_title: String,
    pub paragraph_text: String,
    pub sentence_text: String,
    pub segmentation_method: String,
    pub status: String,
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

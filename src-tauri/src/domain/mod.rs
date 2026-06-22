use serde::Serialize;

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
    pub sentences_imported: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReaderView {
    pub book_id: i64,
    pub book_title: String,
    pub active_toc_node_id: i64,
    pub breadcrumb: Vec<ReaderBreadcrumb>,
    pub toc_nodes: Vec<ReaderTocNode>,
    pub paragraphs: Vec<ReaderParagraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReaderBreadcrumb {
    pub id: i64,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReaderTocNode {
    pub id: i64,
    pub parent_id: Option<i64>,
    pub title: String,
    pub level: i64,
    pub order_index: i64,
    pub content_type: String,
    pub included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReaderParagraph {
    pub id: i64,
    pub toc_node_id: i64,
    pub order_index: i64,
    pub source_href: String,
    pub sentences: Vec<ReaderSentence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReaderSentence {
    pub id: i64,
    pub paragraph_id: i64,
    pub order_index: i64,
    pub text: String,
    pub status: String,
}

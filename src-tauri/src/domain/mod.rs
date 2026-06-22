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

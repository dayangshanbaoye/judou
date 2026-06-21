use std::path::Path;

use judou_lib::ingest::epub::{
    classify_toc_nodes, extract_paragraphs_from_xhtml, locate_opf_path, parse_ncx_toc,
    read_package_document, ContentType,
};
use judou_lib::ingest::import::{import_epub, ImportOptions};
use judou_lib::repo::{BookDraft, ChapterParagraphs, SqliteRepo};

#[test]
fn reference_epub_locates_opf_from_container() {
    let container = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;

    assert_eq!(locate_opf_path(container).unwrap(), "content.opf");
}

#[test]
fn reference_epub_reads_metadata_manifest_and_spine_without_chapters() {
    let package = read_package_document(Path::new(
        "../fixtures/epub/Inside the Box - David Epstein.epub",
    ))
    .unwrap();

    assert_eq!(package.opf_path, "content.opf");
    assert_eq!(package.metadata.title.as_deref(), Some("Inside the Box"));
    assert_eq!(package.metadata.author.as_deref(), Some("David Epstein"));
    assert_eq!(package.metadata.language.as_deref(), Some("en"));
    assert_eq!(package.ncx_href.as_deref(), Some("toc.ncx"));

    let first_five_hrefs: Vec<&str> = package
        .spine
        .iter()
        .take(5)
        .map(|item| item.href.as_str())
        .collect();
    assert_eq!(
        first_five_hrefs,
        vec![
            "titlepage.xhtml",
            "OEBPS/c9.xhtml",
            "OEBPS/cP.xhtml",
            "OEBPS/cZ.xhtml",
            "OEBPS/c2A.xhtml"
        ]
    );
    assert_eq!(package.spine.len(), 45);
}

#[test]
fn reference_epub_parses_ncx_toc_tree_and_classifies_nodes() {
    let package = read_package_document(Path::new(
        "../fixtures/epub/Inside the Box - David Epstein.epub",
    ))
    .unwrap();

    let mut toc = parse_ncx_toc(Path::new(
        "../fixtures/epub/Inside the Box - David Epstein.epub",
    ), package.ncx_href.as_deref().unwrap())
    .unwrap();
    classify_toc_nodes(&mut toc);

    assert_eq!(toc.len(), 16);
    assert_eq!(toc[0].title, "Cover");
    assert_eq!(toc[0].href, "OEBPS/c0.xhtml");
    assert_eq!(toc[0].level, 1);
    assert_eq!(toc[0].content_type, ContentType::Excluded);
    assert!(!toc[0].included);

    let introduction = toc
        .iter()
        .find(|node| node.title.starts_with("Introduction:"))
        .unwrap();
    assert_eq!(introduction.content_type, ContentType::Introduction);
    assert!(introduction.included);

    let part_one = toc
        .iter()
        .find(|node| node.title == "Part I: How Boundaries Create Breakthroughs")
        .unwrap();
    assert_eq!(part_one.content_type, ContentType::TitleOnly);
    assert!(!part_one.included);
    assert_eq!(part_one.children.len(), 3);
    assert_eq!(part_one.children[0].title, "Chapter 1: A World without Limits");
    assert_eq!(part_one.children[0].href, "OEBPS/c6U.xhtml");
    assert_eq!(part_one.children[0].level, 2);
    assert_eq!(part_one.children[0].content_type, ContentType::Body);
    assert!(part_one.children[0].included);

    let notes = toc.iter().find(|node| node.title == "Notes").unwrap();
    assert_eq!(notes.content_type, ContentType::Excluded);
    assert!(!notes.included);
}

#[test]
fn reference_epub_extracts_body_paragraphs_with_trace_anchor() {
    let paragraphs = extract_paragraphs_from_xhtml(
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        "OEBPS/c6U.xhtml",
    )
    .unwrap();

    assert!(paragraphs.len() > 40);
    assert_eq!(paragraphs[0].order_index, 0);
    assert_eq!(paragraphs[0].source_href, "OEBPS/c6U.xhtml");
    assert_eq!(paragraphs[0].source_path, "p[3]");
    assert!(paragraphs[0]
        .clean_text
        .starts_with("Imagine a tech company so visionary"));
    assert!(paragraphs[0].clean_text.contains("A “concept IPO,” they called it."));
    assert!(!paragraphs[0].clean_text.contains("<span"));

    assert_eq!(paragraphs[1].order_index, 1);
    assert!(paragraphs[1]
        .clean_text
        .starts_with("Picture the three founders"));
}

#[test]
fn reference_epub_persists_book_toc_and_paragraph_trace_chain() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    judou_lib::db::run_migrations(&connection).unwrap();

    let package = read_package_document(Path::new(
        "../fixtures/epub/Inside the Box - David Epstein.epub",
    ))
    .unwrap();
    let mut toc = parse_ncx_toc(
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        package.ncx_href.as_deref().unwrap(),
    )
    .unwrap();
    classify_toc_nodes(&mut toc);
    let paragraphs = extract_paragraphs_from_xhtml(
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        "OEBPS/c6U.xhtml",
    )
    .unwrap();

    let repo = SqliteRepo::new(&connection);
    let book_id = repo
        .insert_book_structure(
            &BookDraft {
                title: package.metadata.title.as_deref().unwrap(),
                author: package.metadata.author.as_deref(),
                language: package.metadata.language.as_deref(),
                file_hash: "fixture-inside-the-box-partial",
            },
            &toc,
            &[ChapterParagraphs {
                toc_href: "OEBPS/c6U.xhtml",
                paragraphs,
            }],
        )
        .unwrap();

    let trace = repo
        .find_paragraph_trace(book_id, "OEBPS/c6U.xhtml", 0)
        .unwrap();
    assert_eq!(trace.book_title, "Inside the Box");
    assert_eq!(trace.toc_title, "Chapter 1: A World without Limits");
    assert!(trace.clean_text.starts_with("Imagine a tech company"));
    assert_eq!(repo.count_toc_nodes(book_id).unwrap(), 31);
}

#[test]
fn reference_epub_imports_partial_book_and_returns_report() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    judou_lib::db::run_migrations(&connection).unwrap();

    let imported = import_epub(
        &connection,
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        ImportOptions {
            max_included_chapters: Some(2),
        },
    )
    .unwrap();

    assert_eq!(imported.report.book_id, imported.book_id);
    assert_eq!(imported.report.title, "Inside the Box");
    assert_eq!(imported.report.root_toc_nodes, 16);
    assert_eq!(imported.report.toc_nodes_total, 31);
    assert_eq!(imported.report.included_toc_nodes, 16);
    assert_eq!(imported.report.excluded_toc_nodes, 11);
    assert_eq!(imported.report.chapters_imported, 2);
    assert!(imported.report.paragraphs_imported > 80);

    let repo = SqliteRepo::new(&connection);
    let trace = repo
        .find_paragraph_trace(imported.book_id, "OEBPS/c4A.xhtml", 0)
        .unwrap();
    assert_eq!(trace.toc_title, "Introduction: A Textbook Case of Discovery");
    assert!(trace.clean_text.starts_with("There is, perhaps"));
}

#[test]
fn import_report_can_be_rebuilt_from_database() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    judou_lib::db::run_migrations(&connection).unwrap();

    let imported = import_epub(
        &connection,
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        ImportOptions {
            max_included_chapters: Some(2),
        },
    )
    .unwrap();

    let report = SqliteRepo::new(&connection)
        .get_import_report(imported.book_id)
        .unwrap();

    assert_eq!(report.book_id, imported.book_id);
    assert_eq!(report.title, "Inside the Box");
    assert_eq!(report.root_toc_nodes, 16);
    assert_eq!(report.toc_nodes_total, 31);
    assert_eq!(report.included_toc_nodes, 16);
    assert_eq!(report.title_only_toc_nodes, 4);
    assert_eq!(report.excluded_toc_nodes, 11);
    assert_eq!(report.paragraphs_imported, imported.report.paragraphs_imported);
}

use std::path::Path;

use judou_lib::{
    ingest::import::{import_epub, ImportOptions},
    repo::SqliteRepo,
    segment::{segment_paragraph, segment_paragraph_with_notices, SegmentationMethod},
};

#[test]
fn segment_handles_abbreviations_decimals_and_initials_without_rewriting() {
    let paragraph =
        "Mr. Smith bought 3.14 pies for D. H. Lawrence. He lives in the U.S. now.";

    let sentences = segment_paragraph(paragraph);
    let texts: Vec<&str> = sentences.iter().map(|sentence| sentence.text.as_str()).collect();

    assert_eq!(
        texts,
        vec![
            "Mr. Smith bought 3.14 pies for D. H. Lawrence. ",
            "He lives in the U.S. now."
        ]
    );
    assert_eq!(texts.concat(), paragraph);
    assert!(sentences
        .iter()
        .all(|sentence| sentence.method == SegmentationMethod::Rule));
}

#[test]
fn segment_records_byte_offsets_for_traceability() {
    let paragraph = "Hello world! “Good,” she said. Why?";

    let sentences = segment_paragraph(paragraph);

    assert_eq!(sentences[0].text, "Hello world! ");
    assert_eq!(sentences[0].start_offset, 0);
    assert_eq!(sentences[0].end_offset, "Hello world! ".len());
    assert_eq!(&paragraph[sentences[1].start_offset..sentences[1].end_offset], sentences[1].text);
    assert_eq!(sentences.iter().map(|sentence| sentence.text.as_str()).collect::<String>(), paragraph);
}

#[test]
fn segment_reports_special_handling_notices() {
    let output = segment_paragraph_with_notices("Mr. Smith bought 3.14 pies. He left.");

    assert_eq!(output.sentences.len(), 2);
    assert!(output
        .notices
        .iter()
        .any(|notice| notice.rule_name == "abbreviation"));
    assert!(output
        .notices
        .iter()
        .any(|notice| notice.rule_name == "decimal"));
}

#[test]
fn persisted_sentences_trace_to_paragraph_toc_and_book() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    judou_lib::db::run_migrations(&connection).unwrap();

    let imported = import_epub(
        &connection,
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        ImportOptions {
            max_included_chapters: Some(1),
        },
    )
    .unwrap();
    let repo = SqliteRepo::new(&connection);

    let paragraph = repo
        .find_paragraph(imported.book_id, "OEBPS/c4A.xhtml", 0)
        .unwrap();
    let segmented = segment_paragraph(&paragraph.clean_text);
    repo.insert_sentences(paragraph.id, &segmented).unwrap();

    let trace = repo.find_sentence_trace(imported.book_id, "OEBPS/c4A.xhtml", 0, 0).unwrap();
    assert_eq!(trace.book_title, "Inside the Box");
    assert_eq!(trace.toc_title, "Introduction: A Textbook Case of Discovery");
    assert_eq!(trace.paragraph_text, paragraph.clean_text);
    assert!(trace.sentence_text.starts_with("There is, perhaps"));
    assert_eq!(trace.segmentation_method, "rule");
    assert_eq!(trace.status, "unread");
}

#[test]
fn import_records_segmentation_notices_in_processing_log() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    judou_lib::db::run_migrations(&connection).unwrap();

    let imported = import_epub(
        &connection,
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        ImportOptions {
            max_included_chapters: Some(1),
        },
    )
    .unwrap();

    let repo = SqliteRepo::new(&connection);
    assert!(repo.count_processing_log(imported.book_id, "segment").unwrap() > 0);
}

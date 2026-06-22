use std::path::Path;

use judou_lib::{
    ingest::import::{import_epub, ImportOptions},
    repo::SqliteRepo,
};

fn import_reference_book(connection: &rusqlite::Connection) -> i64 {
    judou_lib::db::run_migrations(connection).unwrap();
    import_epub(
        connection,
        Path::new("../fixtures/epub/Inside the Box - David Epstein.epub"),
        ImportOptions {
            max_included_chapters: Some(2),
        },
    )
    .unwrap()
    .book_id
}

#[test]
fn reader_view_returns_toc_breadcrumb_and_sentence_stream() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let book_id = import_reference_book(&connection);
    let repo = SqliteRepo::new(&connection);

    let view = repo.get_reader_view(book_id, None).unwrap();

    assert_eq!(view.book_id, book_id);
    assert_eq!(view.book_title, "Inside the Box");
    assert!(!view.toc_nodes.is_empty());
    let active_node = view
        .toc_nodes
        .iter()
        .find(|node| node.id == view.active_toc_node_id)
        .unwrap();
    assert_eq!(view.breadcrumb.last().unwrap().title, active_node.title);
    assert!(view
        .paragraphs
        .iter()
        .any(|paragraph| !paragraph.sentences.is_empty()));
    assert!(view
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.sentences.iter())
        .all(|sentence| sentence.status == "unread"));
}

#[test]
fn reader_view_switches_to_clicked_toc_node() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let book_id = import_reference_book(&connection);
    let repo = SqliteRepo::new(&connection);

    let initial_view = repo.get_reader_view(book_id, None).unwrap();
    let target_node = initial_view
        .toc_nodes
        .iter()
        .find(|node| node.title == "Chapter 2: A World with Limits")
        .unwrap();

    let switched_view = repo.get_reader_view(book_id, Some(target_node.id)).unwrap();

    assert_eq!(switched_view.active_toc_node_id, target_node.id);
    assert_eq!(
        switched_view.breadcrumb.last().unwrap().title,
        "Chapter 2: A World with Limits"
    );
    assert!(switched_view
        .paragraphs
        .iter()
        .all(|paragraph| paragraph.toc_node_id == target_node.id));
}

#[test]
fn sentence_status_can_be_marked_understood() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let book_id = import_reference_book(&connection);
    let repo = SqliteRepo::new(&connection);
    let view = repo.get_reader_view(book_id, None).unwrap();
    let first_sentence_id = view.paragraphs[0].sentences[0].id;

    repo.update_sentence_status(first_sentence_id, "understood")
        .unwrap();
    let updated_view = repo
        .get_reader_view(book_id, Some(view.active_toc_node_id))
        .unwrap();

    let updated_sentence = updated_view
        .paragraphs
        .iter()
        .flat_map(|paragraph| paragraph.sentences.iter())
        .find(|sentence| sentence.id == first_sentence_id)
        .unwrap();
    assert_eq!(updated_sentence.status, "understood");
}

#[test]
fn adjacent_sentences_can_be_merged_and_logged_without_rewriting() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let book_id = import_reference_book(&connection);
    let repo = SqliteRepo::new(&connection);
    let view = repo.get_reader_view(book_id, None).unwrap();
    let paragraph = view
        .paragraphs
        .iter()
        .find(|paragraph| paragraph.sentences.len() >= 2)
        .unwrap();
    let first = &paragraph.sentences[0];
    let second = &paragraph.sentences[1];
    let expected_text = format!("{}{}", first.text, second.text);
    let before_log_count = repo.count_processing_log(book_id, "segment").unwrap();

    let merged = repo.merge_sentences(&[first.id, second.id]).unwrap();
    let updated_view = repo
        .get_reader_view(book_id, Some(view.active_toc_node_id))
        .unwrap();
    let updated_paragraph = updated_view
        .paragraphs
        .iter()
        .find(|candidate| candidate.id == paragraph.id)
        .unwrap();

    assert_eq!(merged.text, expected_text);
    assert_eq!(merged.status, "flagged");
    assert_eq!(updated_paragraph.sentences[0].text, expected_text);
    assert_eq!(updated_paragraph.sentences[0].order_index, 0);
    assert_eq!(
        updated_paragraph
            .sentences
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<String>(),
        paragraph
            .sentences
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<String>()
    );
    assert_eq!(
        repo.count_processing_log(book_id, "segment").unwrap(),
        before_log_count + 1
    );
}

#[test]
fn sentence_can_be_split_and_logged_without_rewriting() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();
    let book_id = import_reference_book(&connection);
    let repo = SqliteRepo::new(&connection);
    let view = repo.get_reader_view(book_id, None).unwrap();
    let paragraph = view
        .paragraphs
        .iter()
        .find(|paragraph| !paragraph.sentences.is_empty())
        .unwrap();
    let sentence = &paragraph.sentences[0];
    let split_offset = sentence.text.find(',').unwrap() + 1;
    let expected_left = &sentence.text[..split_offset];
    let expected_right = &sentence.text[split_offset..];
    let before_log_count = repo.count_processing_log(book_id, "segment").unwrap();

    let split = repo.split_sentence(sentence.id, split_offset).unwrap();
    let updated_view = repo
        .get_reader_view(book_id, Some(view.active_toc_node_id))
        .unwrap();
    let updated_paragraph = updated_view
        .paragraphs
        .iter()
        .find(|candidate| candidate.id == paragraph.id)
        .unwrap();

    assert_eq!(split.len(), 2);
    assert_eq!(split[0].text, expected_left);
    assert_eq!(split[1].text, expected_right);
    assert!(split.iter().all(|sentence| sentence.status == "flagged"));
    assert_eq!(
        updated_paragraph
            .sentences
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<String>(),
        paragraph
            .sentences
            .iter()
            .map(|sentence| sentence.text.as_str())
            .collect::<String>()
    );
    assert_eq!(
        repo.count_processing_log(book_id, "segment").unwrap(),
        before_log_count + 1
    );
}

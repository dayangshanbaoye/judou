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

use judou_lib::{
    clock::Clock,
    db,
    llm::{LlmProvider, MockLlm},
    tts::{MockTts, TtsProvider},
};
#[test]
fn migration_creates_initial_schema_tables() {
    let connection = rusqlite::Connection::open_in_memory().unwrap();

    db::run_migrations(&connection).unwrap();

    let table_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert!(table_count >= 20, "expected full schema, got {table_count} objects");

    let default_deck_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM decks WHERE name = '默认'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(default_deck_count, 1);
}

#[tokio::test]
async fn ping_returns_payload_and_job_id() {
    let response = judou_lib::commands::build_ping_response("phase0".to_string());

    assert_eq!(response.message, "pong: phase0");
    assert!(!response.job_id.is_empty());
}

#[test]
fn import_job_response_contains_job_id() {
    let response = judou_lib::commands::build_import_job_response();

    assert!(!response.job_id.is_empty());
}

#[test]
fn import_job_status_tracks_progress_and_completion() {
    let status = judou_lib::commands::ImportJobStatus::running(
        "job-1".to_string(),
        "parse".to_string(),
        35,
        "解析 EPUB 结构与目录".to_string(),
    );

    assert_eq!(status.job_id, "job-1");
    assert_eq!(status.state, "running");
    assert_eq!(status.percent, 35);
    assert_eq!(status.message, "解析 EPUB 结构与目录");
    assert!(status.report.is_none());
    assert!(status.error.is_none());
}

#[tokio::test]
async fn mock_llm_and_tts_are_injectable() {
    let llm = MockLlm::new(serde_json::json!({"ok": true}).to_string());
    let tts = MockTts::new(vec![1, 2, 3]);

    assert_eq!(llm.complete_json("prompt").await.unwrap(), "{\"ok\":true}");
    assert_eq!(tts.synthesize("hello").await.unwrap().audio_bytes, vec![1, 2, 3]);
}

#[test]
fn system_clock_returns_utc_timestamp() {
    let now = judou_lib::clock::SystemClock.now_utc();

    assert!(now.contains('T'));
    assert!(now.ends_with('Z'));
}

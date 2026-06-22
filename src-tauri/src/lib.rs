pub mod clock;
pub mod commands;
pub mod db;
pub mod domain;
pub mod error;
pub mod ingest;
pub mod llm;
pub mod repo;
pub mod segment;
pub mod tts;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .manage(commands::ImportJobStore::default())
        .invoke_handler(tauri::generate_handler![
            commands::ping,
            commands::import_epub,
            commands::get_import_report,
            commands::get_import_job,
            commands::get_scope_nodes,
            commands::confirm_scope,
            commands::list_processing_log,
            commands::promote_log_to_rule,
            commands::get_reader_view,
            commands::update_sentence_status,
            commands::merge_sentences,
            commands::split_sentence
        ])
        .run(tauri::generate_context!())
}

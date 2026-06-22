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
            commands::get_reader_view,
            commands::update_sentence_status
        ])
        .run(tauri::generate_context!())
}

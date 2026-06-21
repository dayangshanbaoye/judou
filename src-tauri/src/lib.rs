pub mod clock;
pub mod commands;
pub mod db;
pub mod error;
pub mod ingest;
pub mod llm;
pub mod tts;

pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![commands::ping])
        .run(tauri::generate_context!())
}

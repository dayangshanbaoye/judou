use serde::Serialize;
use std::{fs, path::PathBuf, thread};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::{
    db,
    error::Result,
    ingest::import::{import_epub as import_epub_file, ImportOptions, ImportReport},
};

pub const PING_EVENT: &str = "ping://pong";
pub const IMPORT_PROGRESS_EVENT: &str = "import://progress";
pub const IMPORT_DONE_EVENT: &str = "import://done";
pub const IMPORT_ERROR_EVENT: &str = "import://error";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PingResponse {
    pub message: String,
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportJobResponse {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportProgressEvent {
    pub job_id: String,
    pub stage: String,
    pub percent: u8,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportDoneEvent {
    pub job_id: String,
    pub book_id: i64,
    pub report: ImportReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportErrorEvent {
    pub job_id: String,
    pub error: ImportEventError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportEventError {
    pub code: &'static str,
    pub message: String,
}

#[tauri::command]
pub async fn ping(app: AppHandle, payload: String) -> Result<PingResponse> {
    let response = build_ping_response(payload);
    app.emit(PING_EVENT, &response)?;
    Ok(response)
}

pub fn build_ping_response(payload: String) -> PingResponse {
    PingResponse {
        message: format!("pong: {payload}"),
        job_id: Uuid::new_v4().to_string(),
    }
}

#[tauri::command]
pub async fn import_epub(app: AppHandle, path: String) -> Result<ImportJobResponse> {
    let response = build_import_job_response();
    let job_id = response.job_id.clone();
    let db_path = app.path().app_data_dir()?.join("judou.sqlite3");
    let epub_path = PathBuf::from(path);
    let worker_app = app.clone();

    emit_import_progress(&app, &job_id, "queued", 0, "导入任务已创建")?;
    thread::spawn(move || {
        let result = run_import_job(&worker_app, &job_id, db_path, epub_path);
        if let Err(error) = result {
            let _ = worker_app.emit(
                IMPORT_ERROR_EVENT,
                ImportErrorEvent {
                    job_id,
                    error: ImportEventError {
                        code: error.code(),
                        message: error.to_string(),
                    },
                },
            );
        }
    });

    Ok(response)
}

pub fn build_import_job_response() -> ImportJobResponse {
    ImportJobResponse {
        job_id: Uuid::new_v4().to_string(),
    }
}

fn run_import_job(
    app: &AppHandle,
    job_id: &str,
    db_path: PathBuf,
    epub_path: PathBuf,
) -> Result<()> {
    emit_import_progress(app, job_id, "migrate", 10, "准备本地数据库")?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let connection = rusqlite::Connection::open(db_path)?;
    db::run_migrations(&connection)?;

    emit_import_progress(app, job_id, "parse", 35, "解析 EPUB 结构与目录")?;
    let imported = import_epub_file(&connection, &epub_path, ImportOptions::full())?;

    emit_import_progress(app, job_id, "done", 100, "导入完成")?;
    app.emit(
        IMPORT_DONE_EVENT,
        ImportDoneEvent {
            job_id: job_id.to_string(),
            book_id: imported.book_id,
            report: imported.report,
        },
    )?;
    Ok(())
}

fn emit_import_progress(
    app: &AppHandle,
    job_id: &str,
    stage: &str,
    percent: u8,
    message: &str,
) -> Result<()> {
    app.emit(
        IMPORT_PROGRESS_EVENT,
        ImportProgressEvent {
            job_id: job_id.to_string(),
            stage: stage.to_string(),
            percent,
            message: message.to_string(),
        },
    )?;
    Ok(())
}

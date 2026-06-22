use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

use crate::{
    db,
    domain::{ImportReport, ReaderSentence, ReaderView},
    error::{JudouError, Result},
    ingest::import::{import_epub as import_epub_file, ImportOptions},
    repo::{ScopeNode, ScopeNodeUpdate},
    segment::segment_paragraph_with_notices,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportJobStatus {
    pub job_id: String,
    pub state: String,
    pub stage: String,
    pub percent: u8,
    pub message: String,
    pub report: Option<ImportReport>,
    pub error: Option<ImportEventError>,
}

impl ImportJobStatus {
    pub fn running(job_id: String, stage: String, percent: u8, message: String) -> Self {
        Self {
            job_id,
            state: "running".to_string(),
            stage,
            percent,
            message,
            report: None,
            error: None,
        }
    }

    fn done(job_id: String, report: ImportReport) -> Self {
        Self {
            job_id,
            state: "done".to_string(),
            stage: "done".to_string(),
            percent: 100,
            message: "导入完成".to_string(),
            report: Some(report),
            error: None,
        }
    }

    fn failed(job_id: String, error: ImportEventError) -> Self {
        Self {
            job_id,
            state: "error".to_string(),
            stage: "error".to_string(),
            percent: 0,
            message: "导入失败".to_string(),
            report: None,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ImportJobStore {
    jobs: Arc<Mutex<HashMap<String, ImportJobStatus>>>,
}

impl ImportJobStore {
    fn set(&self, status: ImportJobStatus) -> Result<()> {
        let mut jobs = self
            .jobs
            .lock()
            .map_err(|_| JudouError::Validation("import job store lock is poisoned".to_string()))?;
        jobs.insert(status.job_id.clone(), status);
        Ok(())
    }

    fn get(&self, job_id: &str) -> Result<Option<ImportJobStatus>> {
        let jobs = self
            .jobs
            .lock()
            .map_err(|_| JudouError::Validation("import job store lock is poisoned".to_string()))?;
        Ok(jobs.get(job_id).cloned())
    }
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
pub async fn import_epub(
    app: AppHandle,
    jobs: State<'_, ImportJobStore>,
    path: String,
) -> Result<ImportJobResponse> {
    let response = build_import_job_response();
    let job_id = response.job_id.clone();
    let db_path = app_database_path(&app)?;
    let epub_path = PathBuf::from(path);
    let worker_app = app.clone();
    let worker_jobs = jobs.inner().clone();

    set_import_progress(&app, jobs.inner(), &job_id, "queued", 0, "导入任务已创建")?;
    thread::spawn(move || {
        let result = run_import_job(&worker_app, &worker_jobs, &job_id, db_path, epub_path);
        if let Err(error) = result {
            let event_error = ImportEventError {
                code: error.code(),
                message: error.to_string(),
            };
            let _ = worker_jobs.set(ImportJobStatus::failed(job_id.clone(), event_error.clone()));
            let _ = worker_app.emit(
                IMPORT_ERROR_EVENT,
                ImportErrorEvent {
                    job_id,
                    error: event_error,
                },
            );
        }
    });

    Ok(response)
}

#[tauri::command]
pub async fn get_import_job(
    jobs: State<'_, ImportJobStore>,
    job_id: String,
) -> Result<ImportJobStatus> {
    jobs.get(&job_id)?
        .ok_or_else(|| JudouError::Validation(format!("import job '{job_id}' not found")))
}

#[tauri::command]
pub async fn get_import_report(app: AppHandle, book_id: i64) -> Result<ImportReport> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.get_import_report(book_id)
}

#[tauri::command]
pub async fn get_scope_nodes(app: AppHandle, book_id: i64) -> Result<Vec<ScopeNode>> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.list_scope_nodes(book_id)
}

#[tauri::command]
pub async fn confirm_scope(
    app: AppHandle,
    book_id: i64,
    nodes: Vec<ScopeNodeUpdate>,
) -> Result<ImportReport> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.confirm_scope(book_id, &nodes, segment_paragraph_with_notices)
}

#[tauri::command]
pub async fn get_reader_view(
    app: AppHandle,
    book_id: i64,
    toc_node_id: Option<i64>,
) -> Result<ReaderView> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.get_reader_view(book_id, toc_node_id)
}

#[tauri::command]
pub async fn update_sentence_status(
    app: AppHandle,
    sentence_id: i64,
    status: String,
) -> Result<ReaderSentence> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.update_sentence_status(sentence_id, &status)
}

#[tauri::command]
pub async fn merge_sentences(app: AppHandle, sentence_ids: Vec<i64>) -> Result<ReaderSentence> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.merge_sentences(&sentence_ids)
}

#[tauri::command]
pub async fn split_sentence(
    app: AppHandle,
    sentence_id: i64,
    split_offset: usize,
) -> Result<Vec<ReaderSentence>> {
    let connection = rusqlite::Connection::open(app_database_path(&app)?)?;
    let repo = crate::repo::SqliteRepo::new(&connection);
    repo.split_sentence(sentence_id, split_offset)
}

pub fn build_import_job_response() -> ImportJobResponse {
    ImportJobResponse {
        job_id: Uuid::new_v4().to_string(),
    }
}

fn run_import_job(
    app: &AppHandle,
    jobs: &ImportJobStore,
    job_id: &str,
    db_path: PathBuf,
    epub_path: PathBuf,
) -> Result<()> {
    set_import_progress(app, jobs, job_id, "migrate", 10, "准备本地数据库")?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let connection = rusqlite::Connection::open(db_path)?;
    db::run_migrations(&connection)?;

    set_import_progress(app, jobs, job_id, "parse", 35, "解析 EPUB 结构与目录")?;
    let imported = import_epub_file(&connection, &epub_path, ImportOptions::full())?;

    jobs.set(ImportJobStatus::done(
        job_id.to_string(),
        imported.report.clone(),
    ))?;
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

fn app_database_path(app: &AppHandle) -> Result<PathBuf> {
    Ok(app.path().app_data_dir()?.join("judou.sqlite3"))
}

fn set_import_progress(
    app: &AppHandle,
    jobs: &ImportJobStore,
    job_id: &str,
    stage: &str,
    percent: u8,
    message: &str,
) -> Result<()> {
    jobs.set(ImportJobStatus::running(
        job_id.to_string(),
        stage.to_string(),
        percent,
        message.to_string(),
    ))?;
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

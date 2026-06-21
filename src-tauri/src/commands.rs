use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::error::Result;

pub const PING_EVENT: &str = "ping://pong";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PingResponse {
    pub message: String,
    pub job_id: String,
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

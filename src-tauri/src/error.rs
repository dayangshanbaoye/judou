use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum JudouError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("epub archive error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("xml error: {0}")]
    Xml(#[from] quick_xml::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("event error: {0}")]
    Event(#[from] tauri::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("provider error: {0}")]
    Provider(String),
}

#[derive(Debug, Serialize)]
pub struct IpcError {
    pub code: &'static str,
    pub message: String,
}

impl Serialize for JudouError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let code = match self {
            JudouError::Db(_) => "DB",
            JudouError::Io(_) => "IO",
            JudouError::Zip(_) | JudouError::Xml(_) => "PARSE",
            JudouError::Event(_) => "UNKNOWN",
            JudouError::Validation(_) => "VALIDATION",
            JudouError::Provider(_) => "UNKNOWN",
        };

        IpcError {
            code,
            message: self.to_string(),
        }
        .serialize(serializer)
    }
}

pub type Result<T> = std::result::Result<T, JudouError>;

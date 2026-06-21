use async_trait::async_trait;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TtsOutput {
    pub audio_bytes: Vec<u8>,
    pub word_timings_json: Option<String>,
}

#[async_trait]
pub trait TtsProvider {
    async fn synthesize(&self, text: &str) -> Result<TtsOutput>;
}

#[derive(Debug, Clone)]
pub struct MockTts {
    audio_bytes: Vec<u8>,
}

impl MockTts {
    pub fn new(audio_bytes: Vec<u8>) -> Self {
        Self { audio_bytes }
    }
}

#[async_trait]
impl TtsProvider for MockTts {
    async fn synthesize(&self, _text: &str) -> Result<TtsOutput> {
        Ok(TtsOutput {
            audio_bytes: self.audio_bytes.clone(),
            word_timings_json: Some("[]".to_string()),
        })
    }
}

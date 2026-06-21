use async_trait::async_trait;

use crate::error::Result;

#[async_trait]
pub trait LlmProvider {
    async fn complete_json(&self, prompt: &str) -> Result<String>;
}

#[derive(Debug, Clone)]
pub struct MockLlm {
    response_json: String,
}

impl MockLlm {
    pub fn new(response_json: String) -> Self {
        Self { response_json }
    }
}

#[async_trait]
impl LlmProvider for MockLlm {
    async fn complete_json(&self, _prompt: &str) -> Result<String> {
        Ok(self.response_json.clone())
    }
}

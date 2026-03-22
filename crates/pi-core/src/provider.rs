use crate::{Error, Message, Result};

pub trait Provider: Send + Sync {
    fn complete(&self, messages: &[Message]) -> Result<ProviderResponse>;
}

#[derive(Debug, Clone)]
pub enum ProviderResponse {
    Message(Message),
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    RequiresInput,
}

pub struct MockProvider {
    responses: Vec<ProviderResponse>,
}

impl MockProvider {
    pub fn new(responses: Vec<ProviderResponse>) -> Self {
        Self { responses }
    }
}

impl Provider for MockProvider {
    fn complete(&self, _messages: &[Message]) -> Result<ProviderResponse> {
        if let Some(response) = self.responses.first().cloned() {
            Ok(response)
        } else {
            Err(Error::Provider("mock provider exhausted".into()))
        }
    }
}

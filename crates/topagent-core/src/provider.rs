use crate::{CancellationToken, Error, Message, ModelRoute, Result};
use std::sync::{Arc, RwLock};

pub trait Provider: Send + Sync {
    fn complete(&self, messages: &[Message], route: &ModelRoute) -> Result<ProviderResponse>;

    fn complete_with_cancel(
        &self,
        messages: &[Message],
        route: &ModelRoute,
        cancel: Option<&CancellationToken>,
    ) -> Result<ProviderResponse> {
        let _ = cancel;
        self.complete(messages, route)
    }
}

#[derive(Debug, Clone)]
pub enum ProviderResponse {
    Message(Message),
    ToolCall {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    ToolCalls(Vec<ToolCallEntry>),
    RequiresInput,
}

#[derive(Debug, Clone)]
pub struct ToolCallEntry {
    pub id: String,
    pub name: String,
    pub args: serde_json::Value,
}

pub struct ScriptedProvider {
    responses: Vec<ProviderResponse>,
    index: Arc<RwLock<usize>>,
}

impl ScriptedProvider {
    pub fn new(responses: Vec<ProviderResponse>) -> Self {
        Self {
            responses,
            index: Arc::new(RwLock::new(0)),
        }
    }
}

impl Provider for ScriptedProvider {
    fn complete(&self, _messages: &[Message], _route: &ModelRoute) -> Result<ProviderResponse> {
        let mut idx = self.index.write().unwrap();
        if let Some(r) = self.responses.get(*idx).cloned() {
            *idx += 1;
            Ok(r)
        } else {
            Err(Error::Provider("scripted provider exhausted".into()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Content, Role};

    #[test]
    fn test_scripted_provider_returns_responses_in_order() {
        let responses = vec![
            ProviderResponse::Message(Message {
                role: Role::Assistant,
                content: Content::Text {
                    text: "first".into(),
                },
            }),
            ProviderResponse::Message(Message {
                role: Role::Assistant,
                content: Content::Text {
                    text: "second".into(),
                },
            }),
        ];
        let provider = ScriptedProvider::new(responses);
        let route = crate::ModelRoute::default();

        let result1 = provider.complete(&[], &route).unwrap();
        let result2 = provider.complete(&[], &route).unwrap();

        assert!(matches!(result1, ProviderResponse::Message(_)));
        assert!(matches!(result2, ProviderResponse::Message(_)));
    }

    #[test]
    fn test_scripted_provider_exhausted_error() {
        let responses = vec![ProviderResponse::Message(Message {
            role: Role::Assistant,
            content: Content::Text {
                text: "only one".into(),
            },
        })];
        let provider = ScriptedProvider::new(responses);
        let route = crate::ModelRoute::default();

        provider.complete(&[], &route).unwrap();
        let result = provider.complete(&[], &route);
        assert!(result.is_err());
    }
}

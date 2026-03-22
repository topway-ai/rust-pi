use crate::tool_spec::ToolSpec;
use crate::{tools::all_specs, Content, Error, Message, Provider, ProviderResponse, Result, Role};
use serde::{Deserialize, Serialize};

const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone)]
pub struct OpenRouterProvider {
    api_key: String,
    model: String,
    client: reqwest::blocking::Client,
    tools: Vec<ToolSpec>,
}

impl OpenRouterProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to create HTTP client"),
            tools: all_specs(),
        }
    }

    pub fn with_tools(
        api_key: impl Into<String>,
        model: impl Into<String>,
        tools: Vec<ToolSpec>,
    ) -> Self {
        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("failed to create HTTP client"),
            tools,
        }
    }
}

impl Provider for OpenRouterProvider {
    fn complete(&self, messages: &[Message]) -> Result<ProviderResponse> {
        let request = self.build_request(messages);
        let response = self
            .client
            .post(format!("{}/chat/completions", OPENROUTER_BASE_URL))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| Error::Provider(format!("request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(Error::Provider(format!("API error {}: {}", status, body)));
        }

        let completion: OpenAIResponse = response
            .json()
            .map_err(|e| Error::Provider(format!("failed to parse response: {}", e)))?;

        self.parse_response(completion)
    }
}

impl OpenRouterProvider {
    pub(crate) fn build_request(&self, messages: &[Message]) -> ChatRequest {
        let messages: Vec<ChatMessage> = messages
            .iter()
            .map(|m| ChatMessage {
                role: match m.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => "system".to_string(),
                    Role::Tool => "tool".to_string(),
                },
                content: match &m.content {
                    Content::Text { text } => text.clone(),
                    Content::ToolRequest { name, args, .. } => {
                        serde_json::json!({"type": "tool_call", "name": name, "args": args})
                            .to_string()
                    }
                    Content::ToolResult { result, .. } => result.clone(),
                },
            })
            .collect();

        let tools: Vec<ToolDefinition> = self
            .tools
            .iter()
            .map(|spec| ToolDefinition {
                tool_type: "function".to_string(),
                function: FunctionDefinition {
                    name: spec.name.to_string(),
                    description: spec.description.to_string(),
                    parameters: spec.input_schema.clone(),
                },
            })
            .collect();

        ChatRequest {
            model: self.model.clone(),
            messages,
            tools: if tools.is_empty() { None } else { Some(tools) },
            tool_choice: Some(serde_json::json!({"type": "auto"})),
        }
    }

    pub(crate) fn parse_response(&self, response: OpenAIResponse) -> Result<ProviderResponse> {
        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::Provider("no choices in response".into()))?;

        let message = choice.message;

        if let Some(tool_calls) = message.tool_calls {
            if let Some(tool_call) = tool_calls.into_iter().next() {
                let id = tool_call.id;
                let function = tool_call.function;
                let name = function.name;
                let args: serde_json::Value = serde_json::from_str(&function.arguments)
                    .map_err(|e| Error::Provider(format!("failed to parse tool args: {}", e)))?;
                return Ok(ProviderResponse::ToolCall { id, name, args });
            }
        }

        let content = message.content.unwrap_or_default();
        Ok(ProviderResponse::Message(Message {
            role: Role::Assistant,
            content: Content::Text { text: content },
        }))
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    tools: Option<Vec<ToolDefinition>>,
    tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDefinition,
}

#[derive(Debug, Serialize)]
struct FunctionDefinition {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OpenAIResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "function")]
    function: FunctionCall,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_spec::ToolSpec;

    #[test]
    fn test_build_request_uses_shared_tool_specs() {
        let specs = vec![
            ToolSpec {
                name: "read",
                description: "read file",
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"path": {"type": "string"}},
                    "required": ["path"]
                }),
            },
            ToolSpec {
                name: "bash",
                description: "run command",
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {"command": {"type": "string"}},
                    "required": ["command"]
                }),
            },
        ];
        let provider = OpenRouterProvider::with_tools("test-key", "test-model", specs);
        let messages = vec![Message::user("test")];
        let request = provider.build_request(&messages);

        let tools = request.tools.unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].function.name, "read");
        assert_eq!(tools[1].function.name, "bash");
    }

    #[test]
    fn test_parse_text_response() {
        let provider = OpenRouterProvider::new("key", "model");
        let response = OpenAIResponse {
            choices: vec![Choice {
                message: ResponseMessage {
                    content: Some("Hello, world!".to_string()),
                    tool_calls: None,
                },
            }],
        };
        let result = provider.parse_response(response).unwrap();
        match result {
            ProviderResponse::Message(msg) => {
                assert_eq!(msg.as_text().unwrap(), "Hello, world!");
            }
            _ => panic!("expected message"),
        }
    }

    #[test]
    fn test_parse_tool_call_response() {
        let provider = OpenRouterProvider::new("key", "model");
        let response = OpenAIResponse {
            choices: vec![Choice {
                message: ResponseMessage {
                    content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call_123".to_string(),
                        function: FunctionCall {
                            name: "read".to_string(),
                            arguments: r#"{"path": "test.txt"}"#.to_string(),
                        },
                    }]),
                },
            }],
        };
        let result = provider.parse_response(response).unwrap();
        match result {
            ProviderResponse::ToolCall { id, name, args } => {
                assert_eq!(id, "call_123");
                assert_eq!(name, "read");
                assert_eq!(args["path"], "test.txt");
            }
            _ => panic!("expected tool call"),
        }
    }

    #[test]
    fn test_parse_malformed_tool_args_fails() {
        let provider = OpenRouterProvider::new("key", "model");
        let response = OpenAIResponse {
            choices: vec![Choice {
                message: ResponseMessage {
                    content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call_123".to_string(),
                        function: FunctionCall {
                            name: "read".to_string(),
                            arguments: "not json".to_string(),
                        },
                    }]),
                },
            }],
        };
        let result = provider.parse_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_tool_order_deterministic() {
        let specs = all_specs();
        let provider = OpenRouterProvider::with_tools("key", "model", specs);
        let messages = vec![Message::user("test")];
        let request1 = provider.build_request(&messages);
        let request2 = provider.build_request(&messages);

        let tools1 = request1.tools.unwrap();
        let tools2 = request2.tools.unwrap();
        assert_eq!(tools1.len(), tools2.len());
        for (t1, t2) in tools1.iter().zip(tools2.iter()) {
            assert_eq!(t1.function.name, t2.function.name);
        }
    }
}

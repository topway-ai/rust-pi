use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: Content,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text {
        text: String,
    },
    ToolRequest {
        id: String,
        name: String,
        args: serde_json::Value,
    },
    ToolResult {
        id: String,
        result: String,
    },
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: Content::Text { text: text.into() },
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::Text { text: text.into() },
        }
    }

    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: Content::Text { text: text.into() },
        }
    }

    pub fn tool_request(
        id: impl Into<String>,
        name: impl Into<String>,
        args: serde_json::Value,
    ) -> Self {
        Self {
            role: Role::Assistant,
            content: Content::ToolRequest {
                id: id.into(),
                name: name.into(),
                args,
            },
        }
    }

    pub fn tool_result(id: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: Content::ToolResult {
                id: id.into(),
                result: result.into(),
            },
        }
    }

    pub fn is_tool_call(&self) -> bool {
        matches!(self.content, Content::ToolRequest { .. })
    }

    pub fn as_text(&self) -> Option<&str> {
        if let Content::Text { text } = &self.content {
            Some(text)
        } else {
            None
        }
    }
}

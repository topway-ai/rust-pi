mod bash;
mod edit;
mod read;
mod write;

pub use bash::BashTool;
pub use edit::EditTool;
pub use read::ReadTool;
pub use write::WriteTool;

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolArg {
    #[serde(flatten)]
    pub fields: HashMap<String, serde_json::Value>,
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, args: serde_json::Value) -> Result<String>;
}

pub fn validate_path(path: &str) -> Result<String> {
    let path = std::path::Path::new(path);
    if path.is_absolute() {
        return Err(Error::InvalidInput("absolute paths not allowed".into()));
    }
    let normalized: String = path
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if normalized.contains("..") {
        return Err(Error::InvalidInput("path traversal not allowed".into()));
    }
    Ok(normalized)
}

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

use super::validate_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadArgs {
    pub path: String,
}

pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReadTool {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Tool for ReadTool {
    fn name(&self) -> &str {
        "read"
    }

    fn description(&self) -> &str {
        "read file contents: args {path: file path}"
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let args: ReadArgs =
            serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let path = validate_path(&args.path)?;
        let full_path = std::path::Path::new(".").join(&path);
        std::fs::read_to_string(&full_path).map_err(|e| {
            Error::ToolFailed(format!("failed to read {}: {}", full_path.display(), e))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[test]
    fn test_read_file() {
        std::fs::write("test_read.txt", "hello world").unwrap();
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "test_read.txt"}));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
        std::fs::remove_file("test_read.txt").unwrap();
    }

    #[test]
    fn test_read_nonexistent() {
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "nonexistent.txt"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_read_path_traversal_rejected() {
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "../etc/passwd"}));
        assert!(result.is_err());
    }
}

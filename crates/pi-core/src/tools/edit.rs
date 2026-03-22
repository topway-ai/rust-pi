use crate::{Error, Result};
use serde::{Deserialize, Serialize};

use super::validate_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    pub path: String,
    pub find: String,
    pub replace: String,
}

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Tool for EditTool {
    fn name(&self) -> &str {
        "edit"
    }

    fn description(&self) -> &str {
        "replace first occurrence of find string with replace: args {path: file path, find: string to find, replace: string to replace}"
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let args: EditArgs =
            serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let path = validate_path(&args.path)?;
        let full_path = std::path::Path::new(".").join(&path);
        let content = std::fs::read_to_string(&full_path).map_err(|e| {
            Error::ToolFailed(format!("failed to read {}: {}", full_path.display(), e))
        })?;

        if !content.contains(&args.find) {
            return Err(Error::ToolFailed(format!(
                "string '{}' not found in {}",
                args.find,
                full_path.display()
            )));
        }

        let new_content = content.replacen(&args.find, &args.replace, 1);
        std::fs::write(&full_path, &new_content).map_err(|e| {
            Error::ToolFailed(format!("failed to write {}: {}", full_path.display(), e))
        })?;

        Ok(format!("replaced 1 occurrence in {}", full_path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;

    #[test]
    fn test_edit_file() {
        std::fs::write("test_edit.txt", "hello world").unwrap();
        let tool = EditTool::new();
        let result = tool.execute(serde_json::json!({
            "path": "test_edit.txt",
            "find": "world",
            "replace": "rust"
        }));
        assert!(result.is_ok(), "{:?}", result);
        let content = std::fs::read_to_string("test_edit.txt").unwrap();
        assert_eq!(content, "hello rust");
        std::fs::remove_file("test_edit.txt").unwrap();
    }

    #[test]
    fn test_edit_not_found() {
        std::fs::write("test_edit2.txt", "hello world").unwrap();
        let tool = EditTool::new();
        let result = tool.execute(serde_json::json!({
            "path": "test_edit2.txt",
            "find": "nonexistent",
            "replace": "replacement"
        }));
        assert!(result.is_err());
        std::fs::remove_file("test_edit2.txt").unwrap();
    }
}

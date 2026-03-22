use crate::{Error, Result};
use serde::{Deserialize, Serialize};

use super::validate_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteArgs {
    pub path: String,
    pub content: String,
}

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl super::Tool for WriteTool {
    fn name(&self) -> &str {
        "write"
    }

    fn description(&self) -> &str {
        "write file contents: args {path: file path, content: string}"
    }

    fn execute(&self, args: serde_json::Value) -> Result<String> {
        let args: WriteArgs =
            serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let path = validate_path(&args.path)?;
        let full_path = std::path::Path::new(".").join(&path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::ToolFailed(format!(
                    "failed to create parent dir for {}: {}",
                    full_path.display(),
                    e
                ))
            })?;
        }
        std::fs::write(&full_path, &args.content).map_err(|e| {
            Error::ToolFailed(format!("failed to write {}: {}", full_path.display(), e))
        })?;
        Ok(format!(
            "wrote {} bytes to {}",
            args.content.len(),
            full_path.display()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use std::io::Read;

    #[test]
    fn test_write_file() {
        let tool = WriteTool::new();
        let result =
            tool.execute(serde_json::json!({"path": "test_output.txt", "content": "hello world"}));
        assert!(result.is_ok(), "{:?}", result);
        let mut content = String::new();
        std::fs::File::open("test_output.txt")
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "hello world");
        std::fs::remove_file("test_output.txt").unwrap();
    }
}

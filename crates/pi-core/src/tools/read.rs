use crate::context::ExecutionContext;
use crate::tool_spec::ToolSpec;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

const MAX_READ_SIZE: usize = 64 * 1024;

fn is_likely_binary(bytes: &[u8]) -> bool {
    bytes.iter().take(8192).any(|&b| b == 0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadArgs {
    pub path: String,
}

#[derive(Clone)]
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

impl crate::tools::Tool for ReadTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec::read()
    }

    fn execute(&self, args: serde_json::Value, ctx: &ExecutionContext) -> Result<String> {
        let args: ReadArgs =
            serde_json::from_value(args).map_err(|e| Error::InvalidInput(e.to_string()))?;
        let full_path = ctx.resolve_path(&args.path)?;

        let bytes = std::fs::read(&full_path).map_err(|e| {
            Error::ToolFailed(format!("failed to read {}: {}", full_path.display(), e))
        })?;

        if is_likely_binary(&bytes) {
            return Err(Error::ReadFailed(format!(
                "binary/non-text file not supported by read tool: {}",
                full_path.display()
            )));
        }

        let original_size = bytes.len();

        if original_size > MAX_READ_SIZE {
            let truncated = &bytes[..MAX_READ_SIZE];
            match String::from_utf8(truncated.to_vec()) {
                Ok(text) => {
                    return Ok(format!(
                        "[ReadTool] File truncated: {} bytes total, showing first {} bytes:\n{}\n\n[ReadTool] File continues... ({} bytes truncated)",
                        original_size,
                        MAX_READ_SIZE,
                        text,
                        original_size - MAX_READ_SIZE
                    ));
                }
                Err(_) => {
                    return Err(Error::ReadFailed(format!(
                        "file is {} bytes (exceeds {} byte limit) and cannot be decoded as UTF-8 text: {}",
                        original_size,
                        MAX_READ_SIZE,
                        full_path.display()
                    )));
                }
            }
        }

        String::from_utf8(bytes).map_err(|_| {
            Error::ReadFailed(format!(
                "file is valid UTF-8 text but read failed: {}",
                full_path.display()
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ExecutionContext;
    use crate::tools::Tool;
    use std::fs;
    use tempfile::TempDir;

    fn test_ctx() -> (ExecutionContext, TempDir) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        (ExecutionContext::new(root), temp)
    }

    #[test]
    fn test_read_file_inside_workspace() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        fs::write(ctx.resolve_path("test.txt").unwrap(), "hello world").unwrap();
        let result = tool.execute(serde_json::json!({"path": "test.txt"}), &ctx);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_read_path_traversal_rejected() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "../etc/passwd"}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_nested_traversal_rejected() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "a/../../b"}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_absolute_path_rejected() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        let result = tool.execute(serde_json::json!({"path": "/etc/passwd"}), &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn test_read_binary_file_rejected() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        fs::write(
            ctx.resolve_path("binary.bin").unwrap(),
            b"\x00\x01\x02binary",
        )
        .unwrap();
        let result = tool.execute(serde_json::json!({"path": "binary.bin"}), &ctx);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("binary"), "expected binary rejection: {}", err);
    }

    #[test]
    fn test_read_truncation() {
        let (ctx, _temp) = test_ctx();
        let tool = ReadTool::new();
        let large_content = "x".repeat(100 * 1024);
        fs::write(ctx.resolve_path("large.txt").unwrap(), &large_content).unwrap();
        let result = tool.execute(serde_json::json!({"path": "large.txt"}), &ctx);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(
            output.contains("truncated"),
            "expected truncation notice: {}",
            output
        );
        assert!(
            output.contains("102400"),
            "expected original size: {}",
            output
        );
    }
}

use crate::context::ToolContext;
use crate::plan::{Plan, TodoStatus};
use crate::tool_spec::ToolSpec;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

const PLANS_DIR: &str = ".topagent/plans";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavePlanArgs {
    pub title: String,
    pub task: Option<String>,
}

pub struct SavePlanTool {
    agent_plan: Option<std::sync::Arc<std::sync::Mutex<Plan>>>,
}

impl SavePlanTool {
    pub fn new() -> Self {
        Self { agent_plan: None }
    }

    pub fn with_plan(plan: std::sync::Arc<std::sync::Mutex<Plan>>) -> Self {
        Self {
            agent_plan: Some(plan),
        }
    }

    pub fn bind_plan(&mut self, plan: std::sync::Arc<std::sync::Mutex<Plan>>) {
        self.agent_plan = Some(plan);
    }
}

impl Default for SavePlanTool {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::tools::Tool for SavePlanTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "save_plan".to_string(),
            description: "Save the current plan to disk for future reuse. Use after creating a useful plan that may be referenced later.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "A short descriptive title for the plan"
                    },
                    "task": {
                        "type": "string",
                        "description": "Optional: the original task/instruction this plan addresses"
                    }
                },
                "required": ["title"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let args: SavePlanArgs = serde_json::from_value(args)
            .map_err(|e| Error::InvalidInput(format!("save_plan: invalid input: {}", e)))?;

        let plan = self
            .agent_plan
            .as_ref()
            .ok_or_else(|| Error::ToolFailed("save_plan: plan not initialized".to_string()))?;

        let plan_guard = plan.lock().map_err(|e| {
            Error::ToolFailed(format!("save_plan: cannot acquire plan lock: {}", e))
        })?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::ToolFailed(format!("save_plan: time error: {}", e)))?
            .as_secs();

        let slug = args
            .title
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
            .collect::<String>()
            .chars()
            .take(40)
            .collect::<String>()
            .replace(' ', "-");

        let filename = format!("{}-{}.md", timestamp, slug);
        let plans_dir = ctx.exec.workspace_root.join(PLANS_DIR);
        std::fs::create_dir_all(&plans_dir).map_err(|e| {
            Error::ToolFailed(format!("save_plan: failed to create directory: {}", e))
        })?;

        let filepath = plans_dir.join(&filename);

        let mut content = String::new();
        content.push_str(&format!("# {}\n\n", args.title));
        content.push_str(&format!("**Saved:** <t:{}>\n\n", timestamp));
        if let Some(task) = args.task {
            content.push_str(&format!("**Task:** {}\n\n", task));
        }
        content.push_str("---\n\n");
        content.push_str("## Plan Items\n\n");

        if plan_guard.is_empty() {
            content.push_str("*No plan items*\n");
        } else {
            for item in plan_guard.items() {
                let status_symbol = match item.status {
                    TodoStatus::Pending => "[ ]",
                    TodoStatus::InProgress => "[>]",
                    TodoStatus::Done => "[x]",
                };
                content.push_str(&format!("- {} {}\n", status_symbol, item.description));
            }
        }

        content.push_str("\n---\n*Saved by topagent*\n");

        std::fs::write(&filepath, &content)
            .map_err(|e| Error::ToolFailed(format!("save_plan: failed to write file: {}", e)))?;

        Ok(format!(
            "Plan saved to .topagent/plans/{}\n\n{}",
            filename, content
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use std::sync::Arc;
    use tempfile::TempDir;

    #[test]
    fn test_save_plan_creates_file() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let exec = crate::context::ExecutionContext::new(root);
        let runtime = crate::runtime::RuntimeOptions::default();
        let ctx = crate::context::ToolContext::new(&exec, &runtime);

        let plan = Arc::new(std::sync::Mutex::new(Plan::new()));
        plan.lock().unwrap().add_item("Test task 1".to_string());
        plan.lock().unwrap().add_item("Test task 2".to_string());

        let tool = SavePlanTool::with_plan(plan);

        let args = serde_json::json!({
            "title": "Test Plan Title"
        });

        let result = tool.execute(args, &ctx);
        assert!(result.is_ok(), "save_plan failed: {:?}", result);
        let output = result.unwrap();
        assert!(output.contains(".topagent/plans/"));
        assert!(output.contains("Test Plan Title"));
        assert!(output.contains("Test task 1"));
        assert!(output.contains("Test task 2"));
    }

    #[test]
    fn test_save_plan_empty_plan() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let exec = crate::context::ExecutionContext::new(root);
        let runtime = crate::runtime::RuntimeOptions::default();
        let ctx = crate::context::ToolContext::new(&exec, &runtime);

        let plan = Arc::new(std::sync::Mutex::new(Plan::new()));
        let tool = SavePlanTool::with_plan(plan);

        let args = serde_json::json!({
            "title": "Empty Plan"
        });

        let result = tool.execute(args, &ctx);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Empty Plan"));
        assert!(output.contains("No plan items"));
    }

    #[test]
    fn test_save_plan_includes_task() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let exec = crate::context::ExecutionContext::new(root);
        let runtime = crate::runtime::RuntimeOptions::default();
        let ctx = crate::context::ToolContext::new(&exec, &runtime);

        let plan = Arc::new(std::sync::Mutex::new(Plan::new()));
        plan.lock().unwrap().add_item("Do something".to_string());

        let tool = SavePlanTool::with_plan(plan);

        let args = serde_json::json!({
            "title": "Plan With Task",
            "task": "Original instruction here"
        });

        let result = tool.execute(args, &ctx);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("**Task:**") && output.contains("Original instruction here"));
    }
}

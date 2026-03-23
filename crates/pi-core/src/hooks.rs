use crate::context::ToolContext;

pub type PreHook = Box<dyn Fn(&str, &serde_json::Value, &ToolContext) -> bool + Send + Sync>;
pub type PostHook =
    Box<dyn Fn(&str, &serde_json::Value, &str, &ToolContext) -> String + Send + Sync>;

pub struct ToolHooks {
    pre_hooks: Vec<PreHook>,
    post_hooks: Vec<PostHook>,
}

impl ToolHooks {
    pub fn new() -> Self {
        Self {
            pre_hooks: Vec::new(),
            post_hooks: Vec::new(),
        }
    }

    pub fn add_pre_hook(&mut self, hook: PreHook) {
        self.pre_hooks.push(hook);
    }

    pub fn add_post_hook(&mut self, hook: PostHook) {
        self.post_hooks.push(hook);
    }

    pub fn run_pre_hooks(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        ctx: &ToolContext,
    ) -> bool {
        for hook in &self.pre_hooks {
            if !hook(tool_name, args, ctx) {
                return false;
            }
        }
        true
    }

    pub fn run_post_hooks(
        &self,
        tool_name: &str,
        args: &serde_json::Value,
        result: &str,
        ctx: &ToolContext,
    ) -> String {
        let mut final_result = result.to_string();
        for hook in &self.post_hooks {
            final_result = hook(tool_name, args, &final_result, ctx);
        }
        final_result
    }

    pub fn is_empty(&self) -> bool {
        self.pre_hooks.is_empty() && self.post_hooks.is_empty()
    }
}

impl Default for ToolHooks {
    fn default() -> Self {
        Self::new()
    }
}

pub struct HookRegistry {
    hooks: std::collections::HashMap<String, ToolHooks>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self {
            hooks: std::collections::HashMap::new(),
        }
    }

    pub fn for_tool(&mut self, tool_name: &str) -> &mut ToolHooks {
        self.hooks.entry(tool_name.to_string()).or_default()
    }

    pub fn get(&self, tool_name: &str) -> Option<&ToolHooks> {
        self.hooks.get(tool_name)
    }

    pub fn global_pre_hooks(&self) -> Vec<&PreHook> {
        self.hooks
            .values()
            .flat_map(|h| h.pre_hooks.iter())
            .collect()
    }

    pub fn global_post_hooks(&self) -> Vec<&PostHook> {
        self.hooks
            .values()
            .flat_map(|h| h.post_hooks.iter())
            .collect()
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::{ExecutionContext, ToolContext};
    use crate::runtime::RuntimeOptions;
    use tempfile::TempDir;

    #[test]
    fn test_tool_hooks_empty_by_default() {
        let hooks = ToolHooks::new();
        assert!(hooks.is_empty());
    }

    #[test]
    fn test_tool_hooks_pre_allows_by_default() {
        let hooks = ToolHooks::new();
        let temp = TempDir::new().unwrap();
        let exec = ExecutionContext::new(temp.path().to_path_buf());
        let runtime = RuntimeOptions::default();
        let ctx = ToolContext::new(&exec, &runtime);
        assert!(hooks.run_pre_hooks("bash", &serde_json::json!({}), &ctx));
    }

    #[test]
    fn test_tool_hooks_post_passes_through() {
        let hooks = ToolHooks::new();
        let temp = TempDir::new().unwrap();
        let exec = ExecutionContext::new(temp.path().to_path_buf());
        let runtime = RuntimeOptions::default();
        let ctx = ToolContext::new(&exec, &runtime);
        let result = hooks.run_post_hooks("bash", &serde_json::json!({}), "original", &ctx);
        assert_eq!(result, "original");
    }

    #[test]
    fn test_tool_hooks_pre_can_block() {
        let mut hooks = ToolHooks::new();
        hooks.add_pre_hook(Box::new(|_name, _args, _ctx| false));

        let temp = TempDir::new().unwrap();
        let exec = ExecutionContext::new(temp.path().to_path_buf());
        let runtime = RuntimeOptions::default();
        let ctx = ToolContext::new(&exec, &runtime);
        assert!(!hooks.run_pre_hooks("bash", &serde_json::json!({}), &ctx));
    }

    #[test]
    fn test_tool_hooks_post_can_modify() {
        let mut hooks = ToolHooks::new();
        hooks.add_post_hook(Box::new(|_name, _args, result, _ctx| {
            format!("modified: {}", result)
        }));

        let temp = TempDir::new().unwrap();
        let exec = ExecutionContext::new(temp.path().to_path_buf());
        let runtime = RuntimeOptions::default();
        let ctx = ToolContext::new(&exec, &runtime);
        let result = hooks.run_post_hooks("bash", &serde_json::json!({}), "original", &ctx);
        assert_eq!(result, "modified: original");
    }

    #[test]
    fn test_hook_registry_for_tool() {
        let mut registry = HookRegistry::new();
        let hooks = registry.for_tool("bash");
        hooks.add_pre_hook(Box::new(|_name, _args, _ctx| false));

        let hooks = registry.get("bash").unwrap();
        let temp = TempDir::new().unwrap();
        let exec = ExecutionContext::new(temp.path().to_path_buf());
        let runtime = RuntimeOptions::default();
        let ctx = ToolContext::new(&exec, &runtime);
        assert!(!hooks.run_pre_hooks("bash", &serde_json::json!({}), &ctx));
    }
}

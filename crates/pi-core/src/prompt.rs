use crate::tool_spec::ToolSpec;

pub fn build_system_prompt(tools: &[ToolSpec]) -> String {
    let mut prompt = String::from(
        "You are a coding assistant that operates within a workspace directory. All file paths are relative to this workspace root.\n\n",
    );
    prompt.push_str("Available tools:\n\n");
    for tool in tools {
        prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
    }
    prompt.push_str("\nOperational guidelines:\n");
    prompt.push_str("- Use relative paths for all file operations (relative to workspace root)\n");
    prompt.push_str("- All paths are validated to stay within the workspace\n");
    prompt.push_str("- Read tool: use to inspect files before modifying\n");
    prompt.push_str(
        "- Write tool: creates or overwrites files; use for new files or full replacements\n",
    );
    prompt.push_str("- Edit tool: for precise string replacements within a file\n");
    prompt.push_str("- Bash tool: execute shell commands; runs in workspace directory\n");
    prompt.push('\n');
    prompt
}

use crate::tool_spec::ToolSpec;

pub const PROJECT_INSTRUCTIONS_SECTION: &str = "## Project Instructions\n\
\
When a PI.md file exists in the workspace root, it contains project-specific guidance that you must follow.\n\
Always check for PI.md at the start of a session and respect its instructions.\n";

pub const PLANNING_SECTION: &str = "## Planning\n\
\nFor non-trivial multi-step tasks, use update_plan to create a plan:\n\
- update_plan with items: [{content, status: \"pending\"|\"in_progress\"|\"done\"}]\n\
- Skip planning for simple one-step tasks\n\
- Update plan item statuses as you complete each step\n\
- When all items are done, the plan can be cleared or left as record\n";

pub const GIT_CONTEXT_SECTION: &str = "## Git Context\n\
\
You have access to git tools for repository awareness:\n\
- git_status: Check for uncommitted changes before making edits\n\
- git_branch: Know your current branch before creating commits or switching context\n\
- git_diff: Review changes before committing or submitting\n\
Use git tools to stay aware of repository state, especially before write operations.\n";

pub fn build_system_prompt(tools: &[ToolSpec], external_tools: &[ToolSpec]) -> String {
    let mut prompt = String::from(
        "You are a coding assistant that operates within a workspace directory. All file paths are relative to this workspace root.\n\n",
    );
    prompt.push_str(PROJECT_INSTRUCTIONS_SECTION);
    prompt.push_str(PLANNING_SECTION);
    prompt.push_str(GIT_CONTEXT_SECTION);

    if !external_tools.is_empty() {
        prompt.push_str("External tools:\n\n");
        for tool in external_tools {
            prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
        }
        prompt.push('\n');
    }

    prompt.push_str("Available tools:\n\n");
    for tool in tools {
        prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
    }
    prompt.push_str("\nOperational guidelines:\n");
    prompt.push_str("- Use relative paths for all file operations (relative to workspace root)\n");
    prompt.push_str("- All paths are validated to stay within the workspace\n");
    prompt.push_str(
        "- Read before edit when exact content matters; this avoids ambiguous replacements\n",
    );
    prompt.push_str(
        "- Read tool: returns text content; may truncate files over 64KB; rejects binary files\n",
    );
    prompt.push_str(
        "- Write tool: creates or overwrites files; use for new files or full replacements\n",
    );
    prompt.push_str("- Edit tool: requires exact old_text and new_text; fails if target is absent or ambiguous\n");
    prompt.push_str("  - For multiple occurrences, set replace_all=true or inspect file first to get unique context\n");
    prompt.push_str("- Bash tool: executes commands locally in workspace; stdout/stderr truncated at 64KB each\n");
    prompt.push_str("- Prefer focused reads and targeted commands; avoid dumping huge outputs\n");
    prompt.push_str(
        "- After tool use, provide a concise final answer rather than repeating tool results\n",
    );
    prompt.push('\n');
    prompt
}

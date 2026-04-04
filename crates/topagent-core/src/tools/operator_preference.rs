use crate::behavior::BehaviorContract;
use crate::context::ToolContext;
use crate::tool_spec::ToolSpec;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MEMORY_ROOT_DIR: &str = ".topagent";
const MEMORY_INDEX_RELATIVE_PATH: &str = ".topagent/MEMORY.md";
const PREFERENCE_FILE_PREFIX: &str = "operator-preference-";
const MAX_KEY_LEN: usize = 48;
const MIN_KEY_LEN: usize = 3;
const MAX_VALUE_LEN: usize = 240;
const MAX_REASON_LEN: usize = 160;
const TRANSIENT_SCOPE_PHRASES: &[&str] = &[
    "this run",
    "this task",
    "this session",
    "for now",
    "right now",
    "temporarily",
    "today only",
    "until this task is done",
    "until this is done",
    "for the current task",
    "current objective",
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperatorPreferenceAction {
    Set,
    Remove,
    List,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreferenceCategory {
    ResponseStyle,
    Workflow,
    Tooling,
    Verification,
}

impl PreferenceCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::ResponseStyle => "response_style",
            Self::Workflow => "workflow",
            Self::Tooling => "tooling",
            Self::Verification => "verification",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManageOperatorPreferenceArgs {
    pub action: OperatorPreferenceAction,
    pub key: Option<String>,
    pub category: Option<PreferenceCategory>,
    pub value: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PreferenceRecord {
    key: String,
    category: PreferenceCategory,
    value: String,
    rationale: Option<String>,
    updated_at: u64,
    relative_file: String,
}

pub struct ManageOperatorPreferenceTool;

impl ManageOperatorPreferenceTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ManageOperatorPreferenceTool {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::tools::Tool for ManageOperatorPreferenceTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "manage_operator_preference".to_string(),
            description: "Create, replace, remove, or list durable operator preferences. Only use for stable cross-run preferences such as response style, verification expectations, or repeatable workflow defaults.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["set", "remove", "list"],
                        "description": "Set or replace a durable preference, remove one, or list all saved preferences"
                    },
                    "key": {
                        "type": "string",
                        "description": "Stable preference identifier, for example concise_final_answers or verify_rust_changes"
                    },
                    "category": {
                        "type": "string",
                        "enum": ["response_style", "workflow", "tooling", "verification"],
                        "description": "Durable preference category. Required for action=set."
                    },
                    "value": {
                        "type": "string",
                        "description": "Short durable preference statement. Required for action=set."
                    },
                    "rationale": {
                        "type": "string",
                        "description": "Optional note explaining why this preference matters across runs."
                    }
                },
                "required": ["action"]
            }),
        }
    }

    fn execute(&self, args: serde_json::Value, ctx: &ToolContext) -> Result<String> {
        let args: ManageOperatorPreferenceArgs = serde_json::from_value(args).map_err(|e| {
            Error::InvalidInput(format!("manage_operator_preference: invalid input: {}", e))
        })?;
        let contract = BehaviorContract::from_runtime_options(ctx.runtime);

        ensure_memory_layout(ctx, &contract)?;

        match args.action {
            OperatorPreferenceAction::Set => set_preference(args, ctx, &contract),
            OperatorPreferenceAction::Remove => remove_preference(args, ctx, &contract),
            OperatorPreferenceAction::List => list_preferences(ctx, &contract),
        }
    }
}

fn set_preference(
    args: ManageOperatorPreferenceArgs,
    ctx: &ToolContext<'_>,
    contract: &BehaviorContract,
) -> Result<String> {
    let raw_key = args.key.as_deref().ok_or_else(|| {
        Error::InvalidInput(
            "manage_operator_preference: key is required for action=set".to_string(),
        )
    })?;
    let key = normalize_key(raw_key)?;
    let category = args.category.ok_or_else(|| {
        Error::InvalidInput(
            "manage_operator_preference: category is required for action=set".to_string(),
        )
    })?;
    let value = normalize_preference_text(
        "value",
        args.value.as_deref().ok_or_else(|| {
            Error::InvalidInput(
                "manage_operator_preference: value is required for action=set".to_string(),
            )
        })?,
        MAX_VALUE_LEN,
        ctx,
    )?;
    let rationale = args
        .rationale
        .as_deref()
        .map(|value| normalize_preference_text("rationale", value, MAX_REASON_LEN, ctx))
        .transpose()?;

    validate_preference_value(&value)?;
    if let Some(reason) = &rationale {
        validate_preference_value(reason)?;
    }

    let topic_dir = topic_directory(ctx, contract)?;
    let relative_file = preference_relative_file(contract, &key);
    let absolute_file = topic_dir.join(preference_file_name(&key));
    let existed = absolute_file.exists();
    let timestamp = current_timestamp()?;
    let record = PreferenceRecord {
        key: key.clone(),
        category,
        value: value.clone(),
        rationale: rationale.clone(),
        updated_at: timestamp,
        relative_file: relative_file.clone(),
    };

    std::fs::write(&absolute_file, render_preference_file(&record)).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to write {}: {}",
            absolute_file.display(),
            e
        ))
    })?;
    upsert_index_entry(ctx, contract, &record)?;

    let verb = if existed { "Updated" } else { "Stored" };
    let mut response = format!(
        "{} operator preference `{}` [{}] in {}",
        verb,
        record.key,
        record.category.as_str(),
        display_preference_file(&record.relative_file)
    );
    response.push_str(&format!("\nPreference: {}", record.value));
    if let Some(reason) = &record.rationale {
        response.push_str(&format!("\nWhy: {}", reason));
    }
    Ok(response)
}

fn remove_preference(
    args: ManageOperatorPreferenceArgs,
    ctx: &ToolContext<'_>,
    contract: &BehaviorContract,
) -> Result<String> {
    let raw_key = args.key.as_deref().ok_or_else(|| {
        Error::InvalidInput(
            "manage_operator_preference: key is required for action=remove".to_string(),
        )
    })?;
    let key = normalize_key(raw_key)?;
    let absolute_file = topic_directory(ctx, contract)?.join(preference_file_name(&key));
    let relative_file = preference_relative_file(contract, &key);
    let removed_file = if absolute_file.exists() {
        std::fs::remove_file(&absolute_file).map_err(|e| {
            Error::ToolFailed(format!(
                "manage_operator_preference: failed to remove {}: {}",
                absolute_file.display(),
                e
            ))
        })?;
        true
    } else {
        false
    };
    let removed_index = remove_index_entry(ctx, contract, &relative_file)?;

    if !removed_file && !removed_index {
        return Ok(format!(
            "No durable operator preference stored for `{}`.",
            key
        ));
    }

    Ok(format!("Removed operator preference `{}`.", key))
}

fn list_preferences(ctx: &ToolContext<'_>, contract: &BehaviorContract) -> Result<String> {
    let topic_dir = topic_directory(ctx, contract)?;
    let mut files = std::fs::read_dir(&topic_dir)
        .map_err(|e| {
            Error::ToolFailed(format!(
                "manage_operator_preference: failed to read {}: {}",
                topic_dir.display(),
                e
            ))
        })?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| is_preference_file(path))
        .collect::<Vec<_>>();
    files.sort();

    if files.is_empty() {
        return Ok("No durable operator preferences stored.".to_string());
    }

    let mut records = files
        .iter()
        .map(|path| parse_preference_file(path.as_path()))
        .collect::<Result<Vec<_>>>()?;
    records.sort_by(|left, right| left.key.cmp(&right.key));

    let mut response = format!("Stored operator preferences ({}):", records.len());
    for record in records {
        response.push_str(&format!(
            "\n- {} [{}] {}",
            record.key,
            record.category.as_str(),
            record.value
        ));
        if let Some(reason) = record.rationale {
            response.push_str(&format!(" | why: {}", reason));
        }
    }
    Ok(response)
}

fn ensure_memory_layout(ctx: &ToolContext<'_>, contract: &BehaviorContract) -> Result<()> {
    let topics_dir = topic_directory(ctx, contract)?;
    std::fs::create_dir_all(&topics_dir).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to create {}: {}",
            topics_dir.display(),
            e
        ))
    })?;

    let index_path = memory_index_path(ctx)?;
    if !index_path.exists() {
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::ToolFailed(format!(
                    "manage_operator_preference: failed to create {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
        std::fs::write(&index_path, contract.render_memory_index_template()).map_err(|e| {
            Error::ToolFailed(format!(
                "manage_operator_preference: failed to write {}: {}",
                index_path.display(),
                e
            ))
        })?;
    }

    Ok(())
}

fn topic_directory(ctx: &ToolContext<'_>, contract: &BehaviorContract) -> Result<PathBuf> {
    ctx.exec.resolve_path(&format!(
        "{}/{}",
        MEMORY_ROOT_DIR, contract.memory.topic_file_relative_dir
    ))
}

fn memory_index_path(ctx: &ToolContext<'_>) -> Result<PathBuf> {
    ctx.exec.resolve_path(MEMORY_INDEX_RELATIVE_PATH)
}

fn preference_relative_file(contract: &BehaviorContract, key: &str) -> String {
    format!(
        "{}/{}",
        contract.memory.topic_file_relative_dir,
        preference_file_name(key)
    )
}

fn preference_file_name(key: &str) -> String {
    format!("{PREFERENCE_FILE_PREFIX}{key}.md")
}

fn display_preference_file(relative_file: &str) -> String {
    format!("{}/{}", MEMORY_ROOT_DIR, relative_file)
}

fn render_preference_file(record: &PreferenceRecord) -> String {
    let mut content = String::new();
    content.push_str(&format!(
        "# Operator Preference: {}\n\n",
        humanize_key(&record.key)
    ));
    content.push_str(&format!("**Key:** {}\n", record.key));
    content.push_str(&format!("**Category:** {}\n", record.category.as_str()));
    content.push_str(&format!("**Updated:** <t:{}>\n\n", record.updated_at));
    content.push_str("## Preference\n\n");
    content.push_str(&record.value);
    content.push_str("\n\n");
    if let Some(rationale) = &record.rationale {
        content.push_str("## Why This Matters\n\n");
        content.push_str(rationale);
        content.push_str("\n\n");
    }
    content.push_str("---\n*Saved by topagent*\n");
    content
}

fn render_index_entry(record: &PreferenceRecord, contract: &BehaviorContract) -> String {
    let note = compact_note(
        &[
            Some(record.value.clone()),
            record
                .rationale
                .as_ref()
                .map(|value| format!("why: {}", compact_text_line(value, 48))),
        ],
        contract.memory.max_index_note_chars,
    );

    format!(
        "- topic: operator preference: {} | file: {} | status: verified | tags: operator, preference, {} | note: {}",
        humanize_key(&record.key),
        record.relative_file,
        record.category.as_str(),
        note
    )
}

fn upsert_index_entry(
    ctx: &ToolContext<'_>,
    contract: &BehaviorContract,
    record: &PreferenceRecord,
) -> Result<()> {
    let index_path = memory_index_path(ctx)?;
    let existing = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|_| contract.render_memory_index_template());
    let mut lines = existing
        .lines()
        .filter(|line| !is_preference_index_line_for_file(line, &record.relative_file))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    if lines.last().is_some_and(|line| !line.trim().is_empty()) {
        lines.push(String::new());
    }
    lines.push(render_index_entry(record, contract));

    let mut rewritten = lines.join("\n");
    rewritten.push('\n');
    std::fs::write(&index_path, rewritten).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to write {}: {}",
            index_path.display(),
            e
        ))
    })
}

fn remove_index_entry(
    ctx: &ToolContext<'_>,
    contract: &BehaviorContract,
    relative_file: &str,
) -> Result<bool> {
    let index_path = memory_index_path(ctx)?;
    if !index_path.exists() {
        return Ok(false);
    }

    let existing = std::fs::read_to_string(&index_path).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to read {}: {}",
            index_path.display(),
            e
        ))
    })?;
    let lines = existing
        .lines()
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    let kept = lines
        .iter()
        .filter(|line| !is_preference_index_line_for_file(line, relative_file))
        .cloned()
        .collect::<Vec<_>>();
    if kept.len() == lines.len() {
        return Ok(false);
    }

    let mut rewritten = if kept.is_empty() {
        contract.render_memory_index_template()
    } else {
        let mut content = kept.join("\n");
        content.push('\n');
        content
    };
    if rewritten.trim().is_empty() {
        rewritten = contract.render_memory_index_template();
    }

    std::fs::write(&index_path, rewritten).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to write {}: {}",
            index_path.display(),
            e
        ))
    })?;
    Ok(true)
}

fn is_preference_index_line_for_file(line: &str, relative_file: &str) -> bool {
    extract_index_field(line, "file").is_some_and(|value| value == relative_file)
}

fn extract_index_field(line: &str, field: &str) -> Option<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('-') {
        return None;
    }

    trimmed
        .trim_start_matches('-')
        .trim()
        .split('|')
        .find_map(|part| {
            let (key, value) = part.split_once(':')?;
            (key.trim().eq_ignore_ascii_case(field)).then_some(value.trim().to_string())
        })
}

fn parse_preference_file(path: &Path) -> Result<PreferenceRecord> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        Error::ToolFailed(format!(
            "manage_operator_preference: failed to read {}: {}",
            path.display(),
            e
        ))
    })?;
    let key = extract_inline_field(&raw, "**Key:**").ok_or_else(|| {
        Error::ToolFailed(format!(
            "manage_operator_preference: missing key in {}",
            path.display()
        ))
    })?;
    let category =
        parse_category(&extract_inline_field(&raw, "**Category:**").ok_or_else(|| {
            Error::ToolFailed(format!(
                "manage_operator_preference: missing category in {}",
                path.display()
            ))
        })?)?;
    let value = extract_markdown_section(&raw, "Preference").ok_or_else(|| {
        Error::ToolFailed(format!(
            "manage_operator_preference: missing preference section in {}",
            path.display()
        ))
    })?;

    Ok(PreferenceRecord {
        key,
        category,
        value,
        rationale: extract_markdown_section(&raw, "Why This Matters"),
        updated_at: extract_saved_timestamp(&raw).unwrap_or_default(),
        relative_file: format!("topics/{}", file_name_or_default(path)),
    })
}

fn parse_category(value: &str) -> Result<PreferenceCategory> {
    match value.trim() {
        "response_style" => Ok(PreferenceCategory::ResponseStyle),
        "workflow" => Ok(PreferenceCategory::Workflow),
        "tooling" => Ok(PreferenceCategory::Tooling),
        "verification" => Ok(PreferenceCategory::Verification),
        other => Err(Error::ToolFailed(format!(
            "manage_operator_preference: unsupported category `{}` in stored preference",
            other
        ))),
    }
}

fn extract_inline_field(contents: &str, prefix: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        line.trim()
            .strip_prefix(prefix)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn extract_markdown_section(contents: &str, heading: &str) -> Option<String> {
    let start_heading = format!("## {heading}");
    let mut lines = Vec::new();
    let mut in_section = false;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed == start_heading {
            in_section = true;
            continue;
        }
        if in_section && trimmed.starts_with("## ") {
            break;
        }
        if in_section {
            lines.push(line);
        }
    }

    let joined = lines.join("\n").trim().to_string();
    (!joined.is_empty()).then_some(joined)
}

fn extract_saved_timestamp(contents: &str) -> Option<u64> {
    contents.lines().find_map(|line| {
        let start = line.find("<t:")?;
        let rest = &line[start + 3..];
        let end = rest.find('>')?;
        rest[..end].parse::<u64>().ok()
    })
}

fn file_name_or_default(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown.md")
        .to_string()
}

fn is_preference_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.starts_with(PREFERENCE_FILE_PREFIX) && name.ends_with(".md"))
}

fn normalize_key(raw: &str) -> Result<String> {
    let mut normalized = String::new();
    let mut just_wrote_separator = false;

    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            just_wrote_separator = false;
            continue;
        }

        if !just_wrote_separator && !normalized.is_empty() {
            normalized.push('_');
            just_wrote_separator = true;
        }
    }

    let normalized = normalized.trim_matches('_').to_string();
    if normalized.len() < MIN_KEY_LEN || normalized.len() > MAX_KEY_LEN {
        return Err(Error::InvalidInput(format!(
            "manage_operator_preference: key must normalize to {}-{} characters",
            MIN_KEY_LEN, MAX_KEY_LEN
        )));
    }
    Ok(normalized)
}

fn normalize_preference_text(
    field: &str,
    value: &str,
    max_len: usize,
    ctx: &ToolContext<'_>,
) -> Result<String> {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return Err(Error::InvalidInput(format!(
            "manage_operator_preference: {} cannot be empty",
            field
        )));
    }
    if collapsed.len() > max_len {
        return Err(Error::InvalidInput(format!(
            "manage_operator_preference: {} must be at most {} characters",
            field, max_len
        )));
    }

    let redacted = ctx.exec.secrets().redact(&collapsed);
    if redacted.as_ref() != collapsed {
        return Err(Error::InvalidInput(format!(
            "manage_operator_preference: {} contains secret-like material and cannot be stored durably",
            field
        )));
    }

    Ok(collapsed)
}

fn validate_preference_value(value: &str) -> Result<()> {
    let lower = value.to_ascii_lowercase();
    if TRANSIENT_SCOPE_PHRASES
        .iter()
        .any(|phrase| lower.contains(phrase))
    {
        return Err(Error::InvalidInput(
            "manage_operator_preference: durable preferences must be stable across runs, not tied to this task or session".to_string(),
        ));
    }
    Ok(())
}

fn humanize_key(key: &str) -> String {
    key.replace(['_', '-'], " ")
}

fn compact_text_line(text: &str, max_bytes: usize) -> String {
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.len() <= max_bytes {
        return collapsed;
    }

    let mut end = max_bytes;
    while end > 0 && !collapsed.is_char_boundary(end) {
        end -= 1;
    }
    let mut limited = collapsed[..end].trim_end().to_string();
    limited.push_str("...");
    limited
}

fn compact_note(parts: &[Option<String>], max_chars: usize) -> String {
    let mut compact = String::new();
    for part in parts.iter().flatten() {
        if part.trim().is_empty() {
            continue;
        }
        if !compact.is_empty() {
            compact.push_str("; ");
        }
        compact.push_str(part.trim());
    }
    compact_text_line(&compact, max_chars)
}

fn current_timestamp() -> Result<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| Error::ToolFailed(format!("manage_operator_preference: time error: {}", e)))
        .map(|duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::Tool;
    use tempfile::TempDir;

    fn create_tool_context() -> (ToolContext<'static>, TempDir) {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let exec = Box::leak(Box::new(crate::context::ExecutionContext::new(root)));
        let runtime = Box::leak(Box::new(crate::runtime::RuntimeOptions::default()));
        (ToolContext::new(exec, runtime), temp)
    }

    #[test]
    fn test_manage_operator_preference_set_list_and_remove() {
        let (ctx, temp) = create_tool_context();
        let tool = ManageOperatorPreferenceTool::new();

        let set_result = tool.execute(
            serde_json::json!({
                "action": "set",
                "key": "concise final answers",
                "category": "response_style",
                "value": "Keep final responses concise and lead with changed files plus verification.",
                "rationale": "The operator reviews many coding runs quickly."
            }),
            &ctx,
        );
        assert!(set_result.is_ok(), "{set_result:?}");
        let set_output = set_result.unwrap();
        assert!(set_output.contains("Stored operator preference `concise_final_answers`"));

        let preference_file = temp
            .path()
            .join(".topagent/topics/operator-preference-concise_final_answers.md");
        assert!(preference_file.is_file());

        let index = std::fs::read_to_string(temp.path().join(".topagent/MEMORY.md")).unwrap();
        assert!(index.contains("operator preference: concise final answers"));
        assert!(index.contains("tags: operator, preference, response_style"));

        let list_output = tool
            .execute(serde_json::json!({ "action": "list" }), &ctx)
            .unwrap();
        assert!(list_output.contains("concise_final_answers [response_style]"));
        assert!(list_output.contains("Keep final responses concise"));

        let remove_output = tool
            .execute(
                serde_json::json!({
                    "action": "remove",
                    "key": "concise_final_answers"
                }),
                &ctx,
            )
            .unwrap();
        assert!(remove_output.contains("Removed operator preference `concise_final_answers`."));
        assert!(!preference_file.exists());

        let index = std::fs::read_to_string(temp.path().join(".topagent/MEMORY.md")).unwrap();
        assert!(!index.contains("operator preference: concise final answers"));
    }

    #[test]
    fn test_manage_operator_preference_set_replaces_existing_entry_without_duplicates() {
        let (ctx, temp) = create_tool_context();
        let tool = ManageOperatorPreferenceTool::new();

        tool.execute(
            serde_json::json!({
                "action": "set",
                "key": "verify_rust_changes",
                "category": "verification",
                "value": "Run cargo test --workspace after meaningful Rust changes."
            }),
            &ctx,
        )
        .unwrap();

        let update_output = tool
            .execute(
                serde_json::json!({
                    "action": "set",
                    "key": "verify_rust_changes",
                    "category": "verification",
                    "value": "Run cargo clippy --workspace --all-targets -- -D warnings and cargo test --workspace after meaningful Rust changes."
                }),
                &ctx,
            )
            .unwrap();
        assert!(update_output.contains("Updated operator preference `verify_rust_changes`"));

        let index = std::fs::read_to_string(temp.path().join(".topagent/MEMORY.md")).unwrap();
        assert_eq!(
            index
                .matches("operator preference: verify rust changes")
                .count(),
            1
        );
        assert!(index.contains("cargo clippy --workspace"));
    }

    #[test]
    fn test_manage_operator_preference_rejects_transient_session_state() {
        let (ctx, temp) = create_tool_context();
        let tool = ManageOperatorPreferenceTool::new();

        let result = tool.execute(
            serde_json::json!({
                "action": "set",
                "key": "current_task_branch",
                "category": "workflow",
                "value": "For this run, stay on the current hotfix branch until this task is done."
            }),
            &ctx,
        );

        assert!(result.is_err());
        let error = format!("{}", result.unwrap_err());
        assert!(error.contains("durable preferences must be stable across runs"));
        assert!(!temp
            .path()
            .join(".topagent/topics/operator-preference-current_task_branch.md")
            .exists());
    }

    #[test]
    fn test_manage_operator_preference_list_empty() {
        let (ctx, _temp) = create_tool_context();
        let tool = ManageOperatorPreferenceTool::new();

        let result = tool
            .execute(serde_json::json!({ "action": "list" }), &ctx)
            .unwrap();
        assert_eq!(result, "No durable operator preferences stored.");
    }
}

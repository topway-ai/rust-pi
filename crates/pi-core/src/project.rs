use crate::Result;
use std::path::Path;

pub const PROJECT_INSTRUCTIONS_FILENAME: &str = "PI.md";

pub fn load_project_instructions(workspace_root: &Path) -> Result<Option<String>> {
    let pi_path = workspace_root.join(PROJECT_INSTRUCTIONS_FILENAME);
    if !pi_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&pi_path).map_err(crate::Error::Io)?;

    Ok(Some(content))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_existing_project_instructions() {
        let temp = TempDir::new().unwrap();
        let pi_content = "# Project Instructions\n\nUse Rust.\n";
        std::fs::write(temp.path().join(PROJECT_INSTRUCTIONS_FILENAME), pi_content).unwrap();

        let result = load_project_instructions(temp.path()).unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap(), pi_content);
    }

    #[test]
    fn test_load_missing_project_instructions() {
        let temp = TempDir::new().unwrap();
        let result = load_project_instructions(temp.path()).unwrap();
        assert!(result.is_none());
    }
}

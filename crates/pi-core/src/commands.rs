use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandStep {
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomCommand {
    pub description: String,
    pub steps: Vec<CommandStep>,
}

#[derive(Debug, Default)]
pub struct CommandRegistry {
    commands: HashMap<String, CustomCommand>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, name: String, command: CustomCommand) {
        self.commands.insert(name, command);
    }

    pub fn get(&self, name: &str) -> Option<&CustomCommand> {
        self.commands.get(name)
    }

    pub fn names(&self) -> Vec<&str> {
        self.commands.keys().map(|s| s.as_str()).collect()
    }

    pub fn load_from_file<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        // TODO: consider logging which commands are being loaded/overwritten
        let content = std::fs::read_to_string(path)?;
        let commands: HashMap<String, CustomCommand> = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        for (name, cmd) in commands {
            self.commands.insert(name, cmd);
        }
        Ok(())
    }

    pub fn load_from_str(&mut self, content: &str) -> Result<(), serde_json::Error> {
        let commands: HashMap<String, CustomCommand> = serde_json::from_str(content)?;
        for (name, cmd) in commands {
            self.commands.insert(name, cmd);
        }
        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    pub fn format_available(&self) -> String {
        if self.commands.is_empty() {
            return String::from("(no custom commands registered)");
        }
        let mut result = String::from("Available custom commands:\n");
        for (name, cmd) in &self.commands {
            result.push_str(&format!("  - {}: {}\n", name, cmd.description));
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_registry_register_and_get() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "test-cmd".to_string(),
            CustomCommand {
                description: "A test command".to_string(),
                steps: vec![CommandStep {
                    tool: "bash".to_string(),
                    args: serde_json::json!({"command": "echo hello"}),
                }],
            },
        );

        let cmd = registry.get("test-cmd").unwrap();
        assert_eq!(cmd.description, "A test command");
        assert_eq!(cmd.steps.len(), 1);
    }

    #[test]
    fn test_command_registry_names() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "cmd1".to_string(),
            CustomCommand {
                description: "1".to_string(),
                steps: vec![],
            },
        );
        registry.register(
            "cmd2".to_string(),
            CustomCommand {
                description: "2".to_string(),
                steps: vec![],
            },
        );

        let names = registry.names();
        assert!(names.contains(&"cmd1"));
        assert!(names.contains(&"cmd2"));
    }

    #[test]
    fn test_command_registry_load_from_str() {
        let mut registry = CommandRegistry::new();
        let json = r#"{
            "greet": {
                "description": "Say hello",
                "steps": [{"tool": "bash", "args": {"command": "echo hello"}}]
            }
        }"#;
        registry.load_from_str(json).unwrap();

        let cmd = registry.get("greet").unwrap();
        assert_eq!(cmd.description, "Say hello");
    }

    #[test]
    fn test_command_registry_format_available() {
        let mut registry = CommandRegistry::new();
        registry.register(
            "cmd1".to_string(),
            CustomCommand {
                description: "First".to_string(),
                steps: vec![],
            },
        );

        let formatted = registry.format_available();
        assert!(formatted.contains("cmd1"));
        assert!(formatted.contains("First"));
    }
}

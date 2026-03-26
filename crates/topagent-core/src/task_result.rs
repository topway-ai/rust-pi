use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskEvidence {
    pub files_changed: Vec<String>,
    pub diff_summary: String,
    pub verification_commands_run: Vec<VerificationCommand>,
    pub unresolved_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCommand {
    pub command: String,
    pub output: String,
    pub exit_code: i32,
    pub succeeded: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskResult {
    pub outcome_summary: String,
    pub evidence: TaskEvidence,
}

impl TaskResult {
    pub fn new(outcome_summary: String) -> Self {
        Self {
            outcome_summary,
            evidence: TaskEvidence::default(),
        }
    }

    pub fn with_files_changed(mut self, files: Vec<String>) -> Self {
        self.evidence.files_changed = files;
        self
    }

    pub fn with_verification_command(mut self, cmd: VerificationCommand) -> Self {
        self.evidence.verification_commands_run.push(cmd);
        self
    }

    pub fn with_verification_commands(mut self, cmds: Vec<VerificationCommand>) -> Self {
        self.evidence.verification_commands_run.extend(cmds);
        self
    }

    pub fn with_unresolved_issue(mut self, issue: String) -> Self {
        self.evidence.unresolved_issues.push(issue);
        self
    }

    pub fn with_unresolved_issues(mut self, issues: Vec<String>) -> Self {
        self.evidence.unresolved_issues.extend(issues);
        self
    }

    pub fn with_diff_summary(mut self, summary: String) -> Self {
        self.evidence.diff_summary = summary;
        self
    }

    pub fn format_proof_of_work(&self) -> String {
        let mut output = String::new();

        if self.evidence.files_changed.is_empty()
            && self.evidence.verification_commands_run.is_empty()
            && self.evidence.unresolved_issues.is_empty()
        {
            return self.outcome_summary.clone();
        }

        output.push_str(&self.outcome_summary);
        output.push_str("\n\n---\n\n## Evidence\n\n");

        if !self.evidence.files_changed.is_empty() {
            output.push_str("### Files Changed\n\n");
            for file in &self.evidence.files_changed {
                output.push_str(&format!("- {}\n", file));
            }
            output.push('\n');

            if !self.evidence.diff_summary.is_empty() {
                output.push_str("### Changes\n\n");
                output.push_str("```\n");
                output.push_str(&self.evidence.diff_summary);
                output.push_str("\n```\n\n");
            }
        }

        if !self.evidence.verification_commands_run.is_empty() {
            output.push_str("### Verification\n\n");
            for vc in &self.evidence.verification_commands_run {
                let status = if vc.succeeded { "PASS" } else { "FAIL" };
                output.push_str(&format!(
                    "- `{}` → exit {} ({})\n",
                    vc.command, vc.exit_code, status
                ));
                if !vc.output.is_empty() {
                    output.push_str("  ```\n  ");
                    output.push_str(&vc.output);
                    output.push_str("\n  ```\n");
                }
            }
            output.push('\n');
        }

        if !self.evidence.unresolved_issues.is_empty() {
            output.push_str("### Unresolved\n\n");
            for issue in &self.evidence.unresolved_issues {
                output.push_str(&format!("- {}\n", issue));
            }
            output.push('\n');
        }

        output.trim_end_matches('\n').to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_result_no_evidence_returns_summary() {
        let result = TaskResult::new("Task completed".to_string());
        let proof = result.format_proof_of_work();
        assert_eq!(proof, "Task completed");
    }

    #[test]
    fn test_task_result_with_files_changed() {
        let result = TaskResult::new("File edited".to_string())
            .with_files_changed(vec!["src/main.rs".to_string()]);
        let proof = result.format_proof_of_work();
        assert!(proof.contains("Files Changed"));
        assert!(proof.contains("src/main.rs"));
    }

    #[test]
    fn test_task_result_with_verification() {
        let cmd = VerificationCommand {
            command: "cargo test".to_string(),
            output: "test result: ok".to_string(),
            exit_code: 0,
            succeeded: true,
        };
        let result = TaskResult::new("Tests passed".to_string()).with_verification_command(cmd);
        let proof = result.format_proof_of_work();
        assert!(proof.contains("Verification"));
        assert!(proof.contains("PASS"));
    }

    #[test]
    fn test_task_result_with_failed_verification() {
        let cmd = VerificationCommand {
            command: "cargo build".to_string(),
            output: "error: failed".to_string(),
            exit_code: 1,
            succeeded: false,
        };
        let result = TaskResult::new("Build failed".to_string()).with_verification_command(cmd);
        let proof = result.format_proof_of_work();
        assert!(proof.contains("FAIL"));
    }

    #[test]
    fn test_task_result_with_unresolved() {
        let result = TaskResult::new("Partial completion".to_string())
            .with_unresolved_issue("Need to fix edge case".to_string());
        let proof = result.format_proof_of_work();
        assert!(proof.contains("Unresolved"));
        assert!(proof.contains("Need to fix edge case"));
    }

    #[test]
    fn test_task_result_full_proof() {
        let cmd = VerificationCommand {
            command: "cargo test".to_string(),
            output: "all tests pass".to_string(),
            exit_code: 0,
            succeeded: true,
        };
        let result = TaskResult::new("Implementation complete".to_string())
            .with_files_changed(vec!["src/lib.rs".to_string()])
            .with_verification_command(cmd)
            .with_unresolved_issue("Documentation not updated".to_string());
        let proof = result.format_proof_of_work();
        assert!(proof.contains("Files Changed"));
        assert!(proof.contains("Verification"));
        assert!(proof.contains("Unresolved"));
    }
}

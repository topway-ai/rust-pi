use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressKind {
    Received,
    Working,
    Stopping,
    Retrying,
    Blocked,
    Completed,
    Failed,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressUpdate {
    pub kind: ProgressKind,
    pub message: String,
}

pub type ProgressCallback = Arc<dyn Fn(ProgressUpdate) + Send + Sync + 'static>;

impl ProgressUpdate {
    pub fn received() -> Self {
        Self {
            kind: ProgressKind::Received,
            message: "Received task.".to_string(),
        }
    }

    pub fn working(message: impl Into<String>) -> Self {
        Self {
            kind: ProgressKind::Working,
            message: message.into(),
        }
    }

    pub fn planning() -> Self {
        Self::working("Planning next steps...")
    }

    pub fn researching() -> Self {
        Self::working("Researching repository...")
    }

    pub fn editing() -> Self {
        Self::working("Editing files...")
    }

    pub fn verifying() -> Self {
        Self::working("Verifying changes...")
    }

    pub fn waiting_for_model(phase: &str) -> Self {
        Self::working(format!("Waiting for model response while {}...", phase))
    }

    pub fn running_tool(tool: impl Into<String>) -> Self {
        Self::working(format!("Running tool: {}", tool.into()))
    }

    pub fn retrying(message: impl Into<String>) -> Self {
        Self {
            kind: ProgressKind::Retrying,
            message: message.into(),
        }
    }

    pub fn stopping() -> Self {
        Self {
            kind: ProgressKind::Stopping,
            message: "Stopping after the current step...".to_string(),
        }
    }

    pub fn retrying_provider(attempt: usize, max_attempts: usize) -> Self {
        Self::retrying(format!(
            "Provider request failed, retrying ({}/{})...",
            attempt, max_attempts
        ))
    }

    pub fn retrying_empty_response(attempt: usize, max_attempts: usize) -> Self {
        Self::retrying(format!(
            "Provider returned an empty response, retrying ({}/{})...",
            attempt, max_attempts
        ))
    }

    pub fn blocked(message: impl Into<String>) -> Self {
        Self {
            kind: ProgressKind::Blocked,
            message: message.into(),
        }
    }

    pub fn completed() -> Self {
        Self {
            kind: ProgressKind::Completed,
            message: "Completed.".to_string(),
        }
    }

    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            kind: ProgressKind::Failed,
            message: format!("Failed: {}", message.into()),
        }
    }

    pub fn stopped() -> Self {
        Self {
            kind: ProgressKind::Stopped,
            message: "Stopped.".to_string(),
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.kind,
            ProgressKind::Completed | ProgressKind::Failed | ProgressKind::Stopped
        )
    }
}

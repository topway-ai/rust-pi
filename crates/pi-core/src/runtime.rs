#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub max_steps: usize,
    pub max_provider_retries: usize,
    pub max_read_bytes: usize,
    pub max_bash_output_bytes: usize,
    pub provider_timeout_secs: u64,
    pub max_messages_before_compaction: usize,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_steps: 50,
            max_provider_retries: 3,
            max_read_bytes: 64 * 1024,
            max_bash_output_bytes: 64 * 1024,
            provider_timeout_secs: 120,
            max_messages_before_compaction: 100,
        }
    }
}

impl RuntimeOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_steps(mut self, max_steps: usize) -> Self {
        self.max_steps = max_steps;
        self
    }

    pub fn with_max_provider_retries(mut self, max_provider_retries: usize) -> Self {
        self.max_provider_retries = max_provider_retries;
        self
    }

    pub fn with_max_read_bytes(mut self, max_read_bytes: usize) -> Self {
        self.max_read_bytes = max_read_bytes;
        self
    }

    pub fn with_max_bash_output_bytes(mut self, max_bash_output_bytes: usize) -> Self {
        self.max_bash_output_bytes = max_bash_output_bytes;
        self
    }

    pub fn with_provider_timeout_secs(mut self, provider_timeout_secs: u64) -> Self {
        self.provider_timeout_secs = provider_timeout_secs;
        self
    }

    pub fn with_max_messages_before_compaction(
        mut self,
        max_messages_before_compaction: usize,
    ) -> Self {
        self.max_messages_before_compaction = max_messages_before_compaction;
        self
    }
}

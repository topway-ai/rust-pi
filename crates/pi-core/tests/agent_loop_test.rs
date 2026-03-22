use pi_core::{
    tools::{BashTool, EditTool, ReadTool, Tool, WriteTool},
    Agent, Message, ProviderResponse,
};
use std::sync::{Arc, RwLock};

struct TestProvider {
    responses: Vec<ProviderResponse>,
    index: Arc<RwLock<usize>>,
}

impl TestProvider {
    fn new(responses: Vec<ProviderResponse>) -> Self {
        Self {
            responses,
            index: Arc::new(RwLock::new(0)),
        }
    }
}

impl pi_core::Provider for TestProvider {
    fn complete(&self, _messages: &[pi_core::Message]) -> pi_core::Result<ProviderResponse> {
        let mut idx = self.index.write().unwrap();
        if let Some(r) = self.responses.get(*idx).cloned() {
            *idx += 1;
            Ok(r)
        } else {
            Err(pi_core::Error::Provider("no more responses".into()))
        }
    }
}

fn make_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadTool::new()) as Box<dyn Tool>,
        Box::new(WriteTool::new()) as Box<dyn Tool>,
        Box::new(EditTool::new()) as Box<dyn Tool>,
        Box::new(BashTool::new()) as Box<dyn Tool>,
    ]
}

#[test]
fn test_agent_returns_final_response() {
    let responses = vec![ProviderResponse::Message(Message::assistant(
        "Hello, how can I help?",
    ))];
    let provider = Box::new(TestProvider::new(responses));
    let mut agent = Agent::new(provider, make_tools());

    let result = agent.run("say hello");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Hello, how can I help?");
}

#[test]
fn test_agent_executes_tool_and_continues() {
    let responses = vec![
        ProviderResponse::ToolCall {
            id: "1".into(),
            name: "bash".into(),
            args: serde_json::json!({"command": "echo hello"}),
        },
        ProviderResponse::Message(Message::assistant("Command executed successfully")),
    ];
    let provider = Box::new(TestProvider::new(responses));
    let mut agent = Agent::new(provider, make_tools());

    let result = agent.run("run a command");
    assert!(result.is_ok());
    assert!(result.unwrap().contains("Command executed successfully"));
}

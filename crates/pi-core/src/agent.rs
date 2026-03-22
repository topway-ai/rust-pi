use crate::context::ExecutionContext;
use crate::prompt;
use crate::session::Session;
use crate::tools::{Tool, ToolRegistry};
use crate::{Error, Message, Provider, ProviderResponse, Result};

pub struct Agent {
    session: Session,
    provider: Box<dyn Provider>,
    tools: ToolRegistry,
}

impl Agent {
    pub fn new(provider: Box<dyn Provider>, tools: Vec<Box<dyn Tool>>) -> Self {
        let mut registry = ToolRegistry::new();
        for tool in tools {
            registry.add(tool);
        }
        Self {
            session: Session::new(),
            provider,
            tools: registry,
        }
    }

    pub fn run(&mut self, ctx: &ExecutionContext, instruction: &str) -> Result<String> {
        self.session.add_message(Message::user(instruction));
        let system_prompt = prompt::build_system_prompt(&self.tools.specs());
        self.session.set_system_prompt(&system_prompt);
        loop {
            let response = self.provider.complete(&self.session.messages())?;
            match response {
                ProviderResponse::Message(msg) => {
                    let text = msg.as_text().map(|s| s.to_string());
                    if let Some(text) = text {
                        if text.is_empty() {
                            return Err(Error::Provider("empty response".into()));
                        }
                        self.session.add_message(msg);
                        return Ok(text);
                    }
                    self.session.add_message(msg);
                }
                ProviderResponse::ToolCall { id, name, args } => {
                    let tool = self
                        .tools
                        .get(&name)
                        .ok_or_else(|| Error::ToolFailed(format!("unknown tool: {}", name)))?;
                    let result = tool.execute(args.clone(), ctx)?;
                    self.session
                        .add_message(Message::tool_request(id.clone(), name, args));
                    self.session.add_message(Message::tool_result(id, result));
                }
                ProviderResponse::RequiresInput => {
                    return Err(Error::Session(
                        "provider requires input, but session is complete".into(),
                    ));
                }
            }
        }
    }
}

use crate::context::ExecutionContext;
use crate::prompt;
use crate::session::Session;
use crate::tools::{Tool, ToolRegistry};
use crate::{Error, Message, Provider, ProviderResponse, Result};

const MAX_STEPS: usize = 50;
const MAX_RETRIES: usize = 3;

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

        let mut steps = 0;
        let mut empty_response_retries = 0;

        loop {
            if steps >= MAX_STEPS {
                return Err(Error::AgentLoop(format!(
                    "max steps ({}) reached without completing task",
                    MAX_STEPS
                )));
            }

            let response = match self.provider.complete(&self.session.messages()) {
                Ok(r) => r,
                Err(e) => {
                    if empty_response_retries >= MAX_RETRIES {
                        return Err(Error::Provider(format!(
                            "provider failed after {} retries: {}",
                            MAX_RETRIES, e
                        )));
                    }
                    empty_response_retries += 1;
                    if empty_response_retries >= MAX_RETRIES {
                        return Err(Error::Provider(format!(
                            "provider failed repeatedly ({} attempts): {}",
                            empty_response_retries, e
                        )));
                    }
                    continue;
                }
            };

            steps += 1;

            match response {
                ProviderResponse::Message(msg) => {
                    let text = msg.as_text().map(|s| s.to_string());
                    if let Some(text) = text {
                        if text.is_empty() {
                            if empty_response_retries >= MAX_RETRIES {
                                return Err(Error::Provider(
                                    "provider returned empty response after max retries".into(),
                                ));
                            }
                            empty_response_retries += 1;
                            continue;
                        }
                        self.session.add_message(msg);
                        return Ok(text);
                    }
                    self.session.add_message(msg);
                }
                ProviderResponse::ToolCall { id, name, args } => {
                    let tool = match self.tools.get(&name) {
                        Some(t) => t,
                        None => {
                            self.session.add_message(Message::tool_request(
                                id.clone(),
                                name.clone(),
                                args,
                            ));
                            self.session.add_message(Message::tool_result(
                                id,
                                format!("error: unknown tool '{}'", name),
                            ));
                            continue;
                        }
                    };
                    let result = match tool.execute(args.clone(), ctx) {
                        Ok(r) => r,
                        Err(e) => {
                            self.session.add_message(Message::tool_request(
                                id.clone(),
                                name.clone(),
                                args,
                            ));
                            self.session.add_message(Message::tool_result(
                                id,
                                format!("error: tool execution failed: {}", e),
                            ));
                            continue;
                        }
                    };
                    self.session
                        .add_message(Message::tool_request(id.clone(), name, args));
                    self.session.add_message(Message::tool_result(id, result));
                    empty_response_retries = 0;
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

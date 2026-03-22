use crate::Message;

#[derive(Debug, Default)]
pub struct Session {
    system_prompt: Option<String>,
    messages: Vec<Message>,
}

impl Session {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_system_prompt(&mut self, prompt: &str) {
        self.system_prompt = Some(prompt.to_string());
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn messages(&self) -> Vec<Message> {
        let mut msgs = Vec::new();
        if let Some(ref sys) = self.system_prompt {
            msgs.push(Message::system(sys));
        }
        msgs.extend(self.messages.clone());
        msgs
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new();
        session.add_message(Message::user("hello"));
        assert_eq!(session.messages().len(), 1);
    }

    #[test]
    fn test_session_with_system_prompt() {
        let mut session = Session::new();
        session.set_system_prompt("you are helpful");
        session.add_message(Message::user("hello"));
        let msgs = session.messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, crate::Role::System);
    }
}

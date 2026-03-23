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

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }

    pub fn compact(&mut self, keep_recent: usize) {
        if self.messages.len() <= keep_recent {
            return;
        }

        let dropped_count = self.messages.len() - keep_recent;
        let messages_clone = self.messages.clone();
        let recent: Vec<Message> = messages_clone.into_iter().rev().take(keep_recent).collect();
        let recent: Vec<Message> = recent.into_iter().rev().collect();

        self.messages.clear();
        self.messages.push(Message::system(format!(
            "[Previous {} messages summarized due to context length.]\nUse tools to re-read files if you need to recall earlier context.",
            dropped_count
        )));
        self.messages.extend(recent);
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

    #[test]
    fn test_session_message_count() {
        let mut session = Session::new();
        assert_eq!(session.message_count(), 0);
        session.add_message(Message::user("hello"));
        assert_eq!(session.message_count(), 1);
        session.add_message(Message::assistant("hi"));
        assert_eq!(session.message_count(), 2);
    }

    #[test]
    fn test_session_compact_keeps_recent_messages() {
        let mut session = Session::new();
        session.set_system_prompt("base prompt");
        for i in 0..20 {
            session.add_message(Message::user(format!("message {}", i)));
        }
        assert_eq!(session.message_count(), 20);

        session.compact(5);

        assert_eq!(session.message_count(), 6);
        let msgs = session.messages();
        assert_eq!(msgs.len(), 7);
        assert_eq!(msgs[0].role, crate::Role::System);
        assert!(msgs[1].as_text().unwrap().contains("summarized"));
        assert!(msgs[1].as_text().unwrap().contains("15"));
        assert!(msgs[2].as_text().unwrap().contains("message 15"));
    }

    #[test]
    fn test_session_compact_does_nothing_when_small() {
        let mut session = Session::new();
        session.add_message(Message::user("hello"));
        session.add_message(Message::user("world"));
        assert_eq!(session.message_count(), 2);

        session.compact(10);

        assert_eq!(session.message_count(), 2);
    }

    #[test]
    fn test_session_compact_preserves_order() {
        let mut session = Session::new();
        session.set_system_prompt("base");
        for i in 0..10 {
            session.add_message(Message::user(format!("msg{}", i)));
        }
        assert_eq!(session.message_count(), 10);

        session.compact(3);

        assert_eq!(session.message_count(), 4);
        let msgs: Vec<_> = session
            .messages()
            .iter()
            .map(|m| m.as_text().unwrap().to_string())
            .collect();
        assert_eq!(msgs[0], "base");
        assert!(msgs[1].contains("summarized"));
        assert!(msgs[2].contains("msg7"));
        assert!(msgs[3].contains("msg8"));
        assert!(msgs[4].contains("msg9"));
    }
}

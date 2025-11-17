use uuid::Uuid;

/// Represents a command issued by the user.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Send a message to the current topic.
    Message(String),
    /// Reply to a specific message.
    Reply { msg_id: Uuid, content: String },
    /// Show help message.
    Help,
    /// An unknown or invalid command.
    Unknown(String),
}

/// Parses a string from the user into a `Command`.
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    if !input.starts_with('/') {
        return Command::Message(input.to_string());
    }

    let mut parts = input.splitn(3, ' ');
    let command = parts.next().unwrap_or("");

    match command {
        "/reply" | "/r" => {
            let msg_id_str = parts.next().unwrap_or("");
            let content = parts.next().unwrap_or("").to_string();

            if content.is_empty() {
                return Command::Unknown(
                    "Reply content cannot be empty. Usage: /reply <msg_id> <content>".to_string(),
                );
            }

            match Uuid::parse_str(msg_id_str) {
                Ok(msg_id) => Command::Reply { msg_id, content },
                Err(_) => Command::Unknown(format!("Invalid message ID for reply: {}", msg_id_str)),
            }
        }
        "/help" | "/h" => Command::Help,
        "/msg" | "/m" => {
            let content = parts.collect::<Vec<&str>>().join(" ");
            if content.is_empty() {
                Command::Unknown("Message content cannot be empty.".to_string())
            } else {
                Command::Message(content)
            }
        }
        _ => Command::Unknown(format!("Unknown command: {}", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        let input = "Hello, world!";
        assert_eq!(
            parse_command(input),
            Command::Message("Hello, world!".to_string())
        );
    }

    #[test]
    fn test_parse_msg_command() {
        let input = "/msg Hello, world!";
        assert_eq!(
            parse_command(input),
            Command::Message("Hello, world!".to_string())
        );
        let input_short = "/m Hello there";
        assert_eq!(
            parse_command(input_short),
            Command::Message("Hello there".to_string())
        );
    }

    #[test]
    fn test_parse_reply_command() {
        let msg_id = Uuid::new_v4();
        let input = format!("/reply {} This is a reply.", msg_id);
        assert_eq!(
            parse_command(&input),
            Command::Reply {
                msg_id,
                content: "This is a reply.".to_string()
            }
        );
    }

    #[test]
    fn test_parse_reply_command_short() {
        let msg_id = Uuid::new_v4();
        let input = format!("/r {} This is a reply.", msg_id);
        assert_eq!(
            parse_command(&input),
            Command::Reply {
                msg_id,
                content: "This is a reply.".to_string()
            }
        );
    }

    #[test]
    fn test_parse_help_command() {
        assert_eq!(parse_command("/help"), Command::Help);
        assert_eq!(parse_command("/h"), Command::Help);
    }

    #[test]
    fn test_parse_unknown_command() {
        let input = "/foo bar";
        assert_eq!(
            parse_command(input),
            Command::Unknown("Unknown command: /foo".to_string())
        );
    }

    #[test]
    fn test_parse_reply_invalid_uuid() {
        let input = "/reply invalid-uuid Test";
        assert_eq!(
            parse_command(input),
            Command::Unknown("Invalid message ID for reply: invalid-uuid".to_string())
        );
    }

    #[test]
    fn test_parse_reply_no_content() {
        let msg_id = Uuid::new_v4();
        let input = format!("/reply {}", msg_id);
        assert_eq!(
            parse_command(&input),
            Command::Unknown(
                "Reply content cannot be empty. Usage: /reply <msg_id> <content>".to_string()
            )
        );
    }
}

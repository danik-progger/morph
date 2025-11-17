use uuid::Uuid;

/// Represents a command issued by the server administrator.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// List clients.
    List(ListScope),
    /// Send a global message to all clients.
    Global(String),
    /// Send a message to a specific topic.
    Topic { topic: String, content: String },
    /// Send a private message to a specific client.
    Private { client_id: Uuid, content: String },
    /// Show help message.
    Help,
    /// Exit the application.
    Exit,
    /// An unknown or invalid command.
    Unknown(String),
}

#[derive(Debug, PartialEq)]
pub enum ListScope {
    All,
    Topic(String),
    Topics,
}

/// Parses a string from the user into a `Command`.
pub fn parse_command(input: &str) -> Command {
    let mut parts = input.trim().splitn(3, ' ');
    let command = parts.next().unwrap_or("").to_lowercase();

    match command.as_str() {
        "/help" | "/h" => Command::Help,
        "/exit" | "/e" => Command::Exit,
        "/list" | "/l" => {
            let scope = parts.next().unwrap_or("all");
            match scope {
                "all" => Command::List(ListScope::All),
                "topics" => Command::List(ListScope::Topics),
                topic => Command::List(ListScope::Topic(topic.to_string())),
            }
        }
        "/global" | "/g" => {
            let content = parts.collect::<Vec<&str>>().join(" ");
            if content.is_empty() {
                Command::Unknown("Global message content cannot be empty.".to_string())
            } else {
                Command::Global(content)
            }
        }
        "/topic" | "/t" => {
            let topic = parts.next().unwrap_or("");
            let content = parts.next().unwrap_or("");
            if topic.is_empty() || content.is_empty() {
                Command::Unknown("Usage: /topic <topic_name> <content>".to_string())
            } else {
                Command::Topic {
                    topic: topic.to_string(),
                    content: content.to_string(),
                }
            }
        }
        "/private" | "/p" => {
            let client_id_str = parts.next().unwrap_or("");
            let content = parts.next().unwrap_or("");
            if client_id_str.is_empty() || content.is_empty() {
                Command::Unknown("Usage: /private <client_id> <content>".to_string())
            } else {
                match Uuid::parse_str(client_id_str) {
                    Ok(client_id) => Command::Private {
                        client_id,
                        content: content.to_string(),
                    },
                    Err(_) => Command::Unknown(format!("Invalid client ID: {}", client_id_str)),
                }
            }
        }
        "" => Command::Unknown("".to_string()), // Ignore empty input
        _ => Command::Unknown(format!("Unknown command: {}", command)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("/help"), Command::Help);
        assert_eq!(parse_command("/h"), Command::Help);
    }

    #[test]
    fn test_parse_exit() {
        assert_eq!(parse_command("/exit"), Command::Exit);
        assert_eq!(parse_command("/e"), Command::Exit);
    }

    #[test]
    fn test_parse_list() {
        assert_eq!(parse_command("/list"), Command::List(ListScope::All));
        assert_eq!(parse_command("/l"), Command::List(ListScope::All));
        assert_eq!(parse_command("/list all"), Command::List(ListScope::All));
        assert_eq!(parse_command("/l all"), Command::List(ListScope::All));
        assert_eq!(
            parse_command("/list topics"),
            Command::List(ListScope::Topics)
        );
        assert_eq!(parse_command("/l topics"), Command::List(ListScope::Topics));
        assert_eq!(
            parse_command("/list general"),
            Command::List(ListScope::Topic("general".to_string()))
        );
        assert_eq!(
            parse_command("/l general"),
            Command::List(ListScope::Topic("general".to_string()))
        );
    }

    #[test]
    fn test_parse_global() {
        assert_eq!(
            parse_command("/global Hello world"),
            Command::Global("Hello world".to_string())
        );
        assert_eq!(
            parse_command("/g Hello world"),
            Command::Global("Hello world".to_string())
        );
        assert_eq!(
            parse_command("/global"),
            Command::Unknown("Global message content cannot be empty.".to_string())
        );
    }

    #[test]
    fn test_parse_topic() {
        assert_eq!(
            parse_command("/topic general Hello"),
            Command::Topic {
                topic: "general".to_string(),
                content: "Hello".to_string()
            }
        );
        assert_eq!(
            parse_command("/t general Hello there"),
            Command::Topic {
                topic: "general".to_string(),
                content: "Hello there".to_string()
            }
        );
        assert_eq!(
            parse_command("/topic general"),
            Command::Unknown("Usage: /topic <topic_name> <content>".to_string())
        );
    }

    #[test]
    fn test_parse_private() {
        let client_id = Uuid::new_v4();
        let input = format!("/private {} Hello", client_id);
        assert_eq!(
            parse_command(&input),
            Command::Private {
                client_id,
                content: "Hello".to_string()
            }
        );

        let input_short = format!("/p {} Hello there", client_id);
        assert_eq!(
            parse_command(&input_short),
            Command::Private {
                client_id,
                content: "Hello there".to_string()
            }
        );

        assert_eq!(
            parse_command("/private foo"),
            Command::Unknown("Usage: /private <client_id> <content>".to_string())
        );

        assert_eq!(
            parse_command("/private 12345 Hello"),
            Command::Unknown("Invalid client ID: 12345".to_string())
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_command(""), Command::Unknown("".to_string()));
        assert_eq!(parse_command(" "), Command::Unknown("".to_string()));
    }
}

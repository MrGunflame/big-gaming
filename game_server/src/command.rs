use std::fmt::Write;

use game_core::command::{
    CommandDescriptor, GameCommand, ParseError, ServerCommand, Token, TokenKind,
};

#[derive(Clone, Debug)]
pub enum Command {
    Server(ServerCommand),
    Game(GameCommand),
    Empty,
}

impl Command {
    pub fn parse(tokens: &[Token<'_>]) -> Result<Self, ParseError> {
        match tokens.get(0) {
            Some(Token::Ident("help")) => {
                Err(ParseError::Msg(display_command_list(None, Self::list())))
            }
            Some(Token::Ident("server")) => match tokens.get(1) {
                Some(Token::Dot) => match tokens.get(2) {
                    Some(Token::Ident("uptime")) => Ok(Self::Server(ServerCommand::Uptime)),
                    Some(Token::Ident("clients")) => Ok(Self::Server(ServerCommand::Clients)),
                    Some(Token::Ident(ident)) => Err(ParseError::Msg(format!(
                        "unknown command {} in server namesapce",
                        ident
                    ))),
                    Some(token) => Err(ParseError::Msg(format!(
                        "expected {}, found {}",
                        TokenKind::Ident,
                        token.kind(),
                    ))),
                    None => Err(ParseError::Msg(format!(
                        "unexpected eof, expected {}",
                        TokenKind::Ident
                    ))),
                },
                Some(token) => Err(ParseError::Msg(format!(
                    "expected {}, found {}",
                    TokenKind::Dot,
                    token.kind()
                ))),
                None => Err(ParseError::Msg(display_command_list(
                    Some("server"),
                    ServerCommand::list(),
                ))),
            },
            Some(Token::Ident("game")) => match tokens.get(1) {
                Some(Token::Dot) => Ok(Self::Game(GameCommand::parse(&tokens[2..])?)),
                Some(token) => Err(ParseError::Msg(format!(
                    "expected {}, found {}",
                    TokenKind::Dot,
                    token.kind()
                ))),
                None => Err(ParseError::Msg(display_command_list(
                    Some("game"),
                    GameCommand::list(),
                ))),
            },
            Some(Token::Ident(ident)) => Err(ParseError::Msg(format!(
                "Unknown command {}\nRun help for a list of commands",
                ident
            ))),
            Some(token) => Err(ParseError::Msg(format!(
                "expected {}, found {}",
                TokenKind::Ident,
                token.kind(),
            ))),
            None => Ok(Self::Empty),
        }

        // dbg!(&tokens);
        // match tokens.first() {
        //     Some(Token::Ident("server")) => Ok(Self::Server(ServerCommand::parse(&tokens[1..])?)),
        //     Some(Token::Ident("game")) => Ok(Self::Game(GameCommand::parse(&tokens[1..])?)),
        //     _ => Err(ParseError::Empty),
        // }
    }

    fn list() -> &'static [CommandDescriptor] {
        &[
            CommandDescriptor {
                name: "server",
                description: "Commands related to server management",
            },
            CommandDescriptor {
                name: "game",
                description: "Commands for manipulation of ingame elements",
            },
        ]
    }
}

fn display_command_list(namespace: Option<&str>, cmds: &[CommandDescriptor]) -> String {
    let mut buf = String::new();
    match namespace {
        Some(namespace) => writeln!(buf, "Available commands for {}", namespace).unwrap(),
        None => writeln!(buf, "Available commands").unwrap(),
    }

    for cmd in cmds {
        writeln!(buf, "{} - {}", cmd.name, cmd.description).unwrap();
    }

    buf
}

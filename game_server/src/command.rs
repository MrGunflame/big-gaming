use game_core::command::{ParseError, ServerCommand, Token};

#[derive(Clone, Debug)]
pub enum Command {
    Server(ServerCommand),
}

impl Command {
    pub fn parse(tokens: &[Token<'_>]) -> Result<Self, ParseError> {
        match tokens.first() {
            Some(Token::Ident("server")) => Ok(Self::Server(ServerCommand::parse(&tokens[1..])?)),
            _ => Err(ParseError::Empty),
        }
    }
}

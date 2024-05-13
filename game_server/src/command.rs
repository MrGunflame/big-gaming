use game_core::command::{GameCommand, ParseError, ServerCommand, Token};

#[derive(Clone, Debug)]
pub enum Command {
    Server(ServerCommand),
    Game(GameCommand),
}

impl Command {
    pub fn parse(tokens: &[Token<'_>]) -> Result<Self, ParseError> {
        match tokens.first() {
            Some(Token::Ident("server")) => Ok(Self::Server(ServerCommand::parse(&tokens[1..])?)),
            Some(Token::Ident("game")) => Ok(Self::Game(GameCommand::parse(&tokens[1..])?)),
            _ => Err(ParseError::Empty),
        }
    }
}

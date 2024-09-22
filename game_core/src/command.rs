//! Command format

use std::fmt::{self, Display, Formatter};

use game_common::entity::EntityId;

#[derive(Copy, Clone, Debug)]
pub struct CommandDescriptor {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Clone, Debug)]
pub enum ServerCommand {
    Uptime,
    Clients,
}

impl ServerCommand {
    pub fn parse(tokens: &[Token<'_>]) -> Result<Self, ParseError> {
        match tokens.first() {
            Some(Token::Ident("uptime")) => Ok(Self::Uptime),
            Some(Token::Ident("clients")) => Ok(Self::Clients),
            _ => Err(ParseError::Empty),
        }
    }

    pub fn list() -> &'static [CommandDescriptor] {
        &[
            CommandDescriptor {
                name: "uptime",
                description: "Show the uptime of the server",
            },
            CommandDescriptor {
                name: "clients",
                description: "List all clients currently connected to the server",
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub enum GameCommand {
    Get(EntityId),
    List,
}

impl GameCommand {
    pub fn parse(tokens: &[Token<'_>]) -> Result<Self, ParseError> {
        match tokens.split_first() {
            Some((&Token::Ident("get"), mut tokens)) => {
                let mut id = None;
                for token in parse_parens(&mut tokens)? {
                    if let Token::Literal(Literal::I64(int)) = token {
                        id = Some(int);
                    } else {
                        return Err(ParseError::Empty);
                    }
                }

                let Some(id) = id else {
                    return Err(ParseError::Empty);
                };

                Ok(Self::Get(EntityId::from_raw(*id as u64)))
            }
            Some((&Token::Ident("list"), _)) => Ok(Self::List),
            _ => Err(ParseError::Empty),
        }
    }

    pub fn list() -> &'static [CommandDescriptor] {
        &[CommandDescriptor {
            name: "get",
            description: "Select an entity",
        }]
    }
}

fn parse_parens<'a>(tokens: &mut &'a [Token<'a>]) -> Result<&'a [Token<'a>], ParseError> {
    let (open, _) = tokens
        .iter()
        .enumerate()
        .find(|(_, t)| **t == Token::OpenParen)
        .ok_or(ParseError::Empty)?;
    let (close, _) = tokens
        .iter()
        .enumerate()
        .find(|(_, t)| **t == Token::CloseParen)
        .ok_or(ParseError::Empty)?;

    // Since `OpenParen != CloseParen` open is never equal to close
    // and `open + 1..close` is always in bounds.
    let contents = &tokens[open + 1..close];

    *tokens = &tokens[close + 1..];
    Ok(contents)
}

pub enum ParseError {
    Empty,
    Msg(String),
}

pub fn tokenize<'a>(input: &'a str) -> Result<Vec<Token<'a>>, TokenizeError> {
    let mut tokens = Vec::new();

    let mut cursor = 0;
    let mut chars = input.char_indices().peekable();

    while let Some((index, char)) = chars.next() {
        let mut slice = &input[cursor..index + char.len_utf8()];

        let token = match char {
            c if c.is_whitespace() => {
                cursor += char.len_utf8();
                continue;
            }
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => Token::Caret,
            '%' => Token::Percent,
            '&' => Token::And,
            '|' => Token::Or,
            '.' => Token::Dot,
            ',' => Token::Comma,
            '(' => Token::OpenParen,
            ')' => Token::CloseParen,
            '"' => {
                loop {
                    match chars.peek().copied() {
                        Some((index, char)) => {
                            slice = &input[cursor..index + char.len_utf8()];
                            chars.next();

                            if char == '"' {
                                break;
                            }
                        }
                        // EOF before closing '"' tag.
                        None => return Err(TokenizeError::MissingTerminator(cursor, '"')),
                    }
                }

                // slice should contain at least two '"' tags.
                cursor += slice.len() - 2;
                tokens.push(Token::Literal(Literal::String(&slice[1..slice.len() - 1])));
                continue;
            }
            '0'..='9' => {
                let base = 10;
                let mut num = (char as u8 - b'0') as i64;

                loop {
                    match chars.peek().copied() {
                        Some((index, char)) if matches!(char, '0'..='9') => {
                            num = num * base + (char as u8 - b'0') as i64;
                            slice = &input[cursor..index + char.len_utf8()];
                            chars.next();
                        }
                        _ => break,
                    }
                }

                cursor += slice.len();
                tokens.push(Token::Literal(Literal::I64(num)));
                continue;
            }
            _ => {
                loop {
                    match chars.peek() {
                        Some((index, char)) if is_valid_ident(*char) => {
                            slice = &input[cursor..*index + char.len_utf8()];
                            chars.next();
                        }
                        _ => break,
                    }
                }

                cursor += slice.len();
                tokens.push(Token::Ident(slice));
                continue;
            }
        };

        cursor += char.len_utf8();
        tokens.push(token);
    }

    Ok(tokens)
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Token<'a> {
    Ident(&'a str),
    Literal(Literal<'a>),
    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `^`
    Caret,
    /// `%`
    Percent,
    /// `&`
    And,
    /// `|`
    Or,
    /// `.`
    Dot,
    /// `,`
    Comma,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
}

impl<'a> Token<'a> {
    pub const fn kind(&self) -> TokenKind {
        match self {
            Self::Ident(_) => TokenKind::Ident,
            Self::Literal(_) => TokenKind::Literal,
            Self::Plus => TokenKind::Plus,
            Self::Minus => TokenKind::Minus,
            Self::Star => TokenKind::Star,
            Self::Slash => TokenKind::Slash,
            Self::Caret => TokenKind::Caret,
            Self::Percent => TokenKind::Percent,
            Self::And => TokenKind::And,
            Self::Or => TokenKind::Or,
            Self::Dot => TokenKind::Dot,
            Self::Comma => TokenKind::Comma,
            Self::OpenParen => TokenKind::OpenParen,
            Self::CloseParen => TokenKind::CloseParen,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident,
    Literal,
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Percent,
    And,
    Or,
    Dot,
    Comma,
    OpenParen,
    CloseParen,
}

impl Display for TokenKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ident => "identifier",
            Self::Literal => "literal",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Caret => "^",
            Self::Percent => "%",
            Self::And => "&",
            Self::Or => "|",
            Self::Dot => ".",
            Self::Comma => ",",
            Self::OpenParen => "(",
            Self::CloseParen => ")",
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum Literal<'a> {
    String(&'a str),
    I64(i64),
    F64(f64),
    Bool(bool),
}

fn is_valid_ident(c: char) -> bool {
    !matches!(
        c,
        '+' | '-' | '*' | '/' | '^' | '%' | '&' | '|' | '.' | ',' | '(' | ')' | '"' | '\''
    ) && !c.is_whitespace()
}

#[derive(Clone, Debug)]
pub enum TokenizeError {
    MissingTerminator(usize, char),
}

#[cfg(test)]
mod tests {
    use super::{tokenize, Literal, Token};

    #[test]
    fn tokenize_idents_whitespace_separated() {
        let input = "a b c";
        let output = [Token::Ident("a"), Token::Ident("b"), Token::Ident("c")];

        assert_eq!(tokenize(input).unwrap(), output);
    }

    #[test]
    fn tokenize_idents_operator_separated() {
        let input = "a+b-c";
        let output = [
            Token::Ident("a"),
            Token::Plus,
            Token::Ident("b"),
            Token::Minus,
            Token::Ident("c"),
        ];

        assert_eq!(tokenize(input).unwrap(), output);
    }

    #[test]
    fn tokenize_idents_unicode() {
        let input = "öä&ßü";
        let output = [Token::Ident("öä"), Token::And, Token::Ident("ßü")];

        assert_eq!(tokenize(input).unwrap(), output);
    }

    #[test]
    fn tokenize_string_literal() {
        let input = "\"Hello World\"";
        let output = [Token::Literal(Literal::String("Hello World"))];

        assert_eq!(tokenize(input).unwrap(), output);
    }

    #[test]
    fn tokenize_int_literal() {
        let input = "123";
        let output = [Token::Literal(Literal::I64(123))];

        assert_eq!(tokenize(input).unwrap(), output);
    }

    #[test]
    fn tokenize_parens() {
        let input = "game.get(123)";
        let output = [
            Token::Ident("game"),
            Token::Dot,
            Token::Ident("get"),
            Token::OpenParen,
            Token::Literal(Literal::I64(123)),
            Token::CloseParen,
        ];

        assert_eq!(tokenize(input).unwrap(), output);
    }
}

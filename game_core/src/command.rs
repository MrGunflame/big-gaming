//! Command format

use game_common::entity::EntityId;

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
}

#[derive(Clone, Debug)]
pub enum GameCommand {
    Get(EntityId),
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
            _ => Err(ParseError::Empty),
        }
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

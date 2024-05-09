//! Command format

pub enum GameCommands {}

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
        '+' | '-' | '*' | '/' | '^' | '%' | '&' | '|' | '"' | '\''
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
}

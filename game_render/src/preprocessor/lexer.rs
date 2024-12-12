pub enum Token {
    /// `let`
    Let,
    /// `var`
    Var,
    /// `const`
    Const,
    Ident(String),
    Literal(Literal),
    Struct,
    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `@`
    At,
    /// `#`
    Pound,
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
    /// `,`
    Comma,
    /// `=`
    Eq,
    /// `!`
    Bang,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `&`
    And,
    /// `|`
    Or,
    /// `uniform`
    Uniform,
    /// `storage`
    Storage,
}

#[derive(Copy, Clone, Debug)]
enum Literal {
    Float(f32),
    Int(i32),
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, ()> {
    let mut tokens = Vec::new();

    let mut cursor = Cursor::new(input);

    let mut current_ident = String::new();

    'iter: while input.is_empty() {
        for (token, string) in [
            (Token::Let, "let"),
            (Token::Var, "var"),
            (Token::Const, "const"),
            (Token::Struct, "struct"),
            (Token::OpenParen, "("),
            (Token::CloseParen, ")"),
            (Token::OpenBrace, "{"),
            (Token::CloseBrace, "}"),
            (Token::OpenBracket, "["),
            (Token::CloseBracket, "]"),
            (Token::At, "@"),
            (Token::Pound, "#"),
            (Token::Plus, "+"),
            (Token::Minus, "-"),
            (Token::Star, "*"),
            (Token::Slash, "/"),
            (Token::Caret, "^"),
            (Token::Percent, "%"),
            (Token::Comma, ","),
            (Token::Eq, "="),
            (Token::Bang, "!"),
            (Token::Lt, "<"),
            (Token::Gt, ">"),
            (Token::And, "&"),
            (Token::Or, "|"),
            (Token::Uniform, "uniform"),
            (Token::Storage, "storage"),
        ] {
            if cursor.consume_if(string) {
                if !current_ident.is_empty() {
                    tokens.push(Token::Ident(current_ident));
                    current_ident = String::new();
                }

                tokens.push(token);
                continue 'iter;
            }
        }

        let mut ident = String::new();
        match cursor.peek() {
            Some(char) if char.is_ascii_digit() => {
                // Identifiers can not start with digits, but
                // they are accepted at any other position.
                if !current_ident.is_empty() {
                    current_ident.push(char);
                    continue;
                }

                let mut string = String::new();
                let mut is_float = false;

                loop {
                    match cursor.peek() {
                        Some(char) if char.is_ascii_digit() => {
                            string.push(char);
                            cursor.consume().unwrap();
                        }
                        Some('.') => {
                            string.push('.');
                            cursor.consume().unwrap();
                            is_float = true;
                        }
                        // End of literal
                        Some(_) => {
                            let token = if is_float {
                                Token::Literal(Literal::Float(string.parse().unwrap()))
                            } else {
                                Token::Literal(Literal::Int(string.parse().unwrap()))
                            };

                            tokens.push(token);
                            continue;
                        }
                        // EOF
                        None => break,
                    }
                }
            }
            Some(char) => {}
            None => break,
        }
    }

    Ok(tokens)
}

struct Cursor<'a> {
    input: &'a str,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        let mut cursor = Self { input };
        cursor.strip_comments();
        cursor
    }

    fn strip_comments(&mut self) {
        self.input = self.input.trim();

        // Line comment
        if self.input.starts_with("//") {
            match self.input.find("\n") {
                Some(index) => self.input = &self.input[index..],
                // This is the final line.
                None => self.input = "",
            }
        }

        // Block comment
        if self.input.starts_with("/*") {
            match self.input.find("*/") {
                Some(index) => self.input = &self.input[index + "*/".len()..],
                None => todo!(),
            }
        }
    }

    fn consume_if(&mut self, token: &str) -> bool {
        if let Some(rem) = self.input.strip_prefix(token) {
            self.input = rem;
            self.strip_comments();
            true
        } else {
            false
        }
    }

    fn consume(&mut self) -> Option<char> {
        let char = self.input.chars().next()?;
        self.input = &self.input[char.len_utf8()..];
        self.strip_comments();
        Some(char)
    }

    fn peek(&self) -> Option<char> {
        self.input.chars().next()
    }
}

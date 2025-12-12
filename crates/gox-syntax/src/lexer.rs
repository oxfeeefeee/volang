//! Lexer for GoX source code.

use crate::token::{Span, Token, TokenKind};

/// Lexer for GoX source code with automatic semicolon insertion.
pub struct Lexer<'a> {
    input: &'a str,
    pos: usize,
    ch: Option<char>,
    prev_kind: Option<TokenKind>,
    at_newline: bool,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer for the given input.
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Self {
            input,
            pos: 0,
            ch: None,
            prev_kind: None,
            at_newline: false,
        };
        lexer.read_char();
        lexer
    }

    /// Read the next character.
    fn read_char(&mut self) {
        self.ch = self.input[self.pos..].chars().next();
        if let Some(c) = self.ch {
            self.pos += c.len_utf8();
        }
    }

    /// Peek at the next character without consuming.
    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    /// Skip whitespace and track newlines for semicolon insertion.
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.ch {
            if c == '\n' {
                self.at_newline = true;
                self.read_char();
            } else if c.is_whitespace() {
                self.read_char();
            } else if c == '/' && self.peek_char() == Some('/') {
                // Line comment
                while self.ch.is_some() && self.ch != Some('\n') {
                    self.read_char();
                }
            } else if c == '/' && self.peek_char() == Some('*') {
                // Block comment
                self.read_char(); // consume /
                self.read_char(); // consume *
                while self.ch.is_some() {
                    if self.ch == Some('*') && self.peek_char() == Some('/') {
                        self.read_char(); // consume *
                        self.read_char(); // consume /
                        break;
                    }
                    if self.ch == Some('\n') {
                        self.at_newline = true;
                    }
                    self.read_char();
                }
            } else {
                break;
            }
        }
    }

    /// Check if automatic semicolon should be inserted.
    fn should_insert_semi(&self) -> bool {
        match &self.prev_kind {
            Some(k) => matches!(
                k,
                TokenKind::Ident(_)
                    | TokenKind::Int(_)
                    | TokenKind::Float(_)
                    | TokenKind::String(_)
                    | TokenKind::True
                    | TokenKind::False
                    | TokenKind::Nil
                    | TokenKind::Break
                    | TokenKind::Continue
                    | TokenKind::Return
                    | TokenKind::Fallthrough
                    | TokenKind::RParen
                    | TokenKind::RBracket
                    | TokenKind::RBrace
            ),
            None => false,
        }
    }

    /// Read an identifier or keyword.
    fn read_ident(&mut self) -> TokenKind {
        let start = self.pos - self.ch.unwrap().len_utf8();
        while let Some(c) = self.ch {
            if c.is_alphanumeric() || c == '_' {
                self.read_char();
            } else {
                break;
            }
        }
        // self.pos points to the character AFTER self.ch, but self.ch is the first
        // non-ident character. So we need to calculate the end correctly.
        let end = match self.ch {
            Some(c) => self.pos - c.len_utf8(),
            None => self.pos,
        };
        let text = &self.input[start..end];
        Self::lookup_ident(text)
    }

    /// Read a number (integer or float).
    fn read_number(&mut self) -> TokenKind {
        let start = self.pos - self.ch.unwrap().len_utf8();
        let mut is_float = false;

        while let Some(c) = self.ch {
            if c.is_ascii_digit() {
                self.read_char();
            } else if c == '.' && !is_float {
                if let Some(next) = self.peek_char() {
                    if next.is_ascii_digit() {
                        is_float = true;
                        self.read_char(); // consume .
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Calculate end position correctly
        let end = match self.ch {
            Some(c) => self.pos - c.len_utf8(),
            None => self.pos,
        };
        let text = &self.input[start..end];
        if is_float {
            TokenKind::Float(text.parse().unwrap_or(0.0))
        } else {
            TokenKind::Int(text.parse().unwrap_or(0))
        }
    }

    /// Read a string literal.
    fn read_string(&mut self) -> TokenKind {
        self.read_char(); // consume opening "
        let mut result = String::new();

        loop {
            match self.ch {
                None => return TokenKind::UnterminatedString,
                Some('"') => {
                    self.read_char();
                    return TokenKind::String(result);
                }
                Some('\\') => {
                    self.read_char();
                    match self.ch {
                        Some('n') => {
                            result.push('\n');
                            self.read_char();
                        }
                        Some('t') => {
                            result.push('\t');
                            self.read_char();
                        }
                        Some('\\') => {
                            result.push('\\');
                            self.read_char();
                        }
                        Some('"') => {
                            result.push('"');
                            self.read_char();
                        }
                        Some(c) => {
                            result.push(c);
                            self.read_char();
                        }
                        None => return TokenKind::UnterminatedString,
                    }
                }
                Some('\n') => return TokenKind::UnterminatedString,
                Some(c) => {
                    result.push(c);
                    self.read_char();
                }
            }
        }
    }

    /// Look up keyword or return identifier.
    fn lookup_ident(ident: &str) -> TokenKind {
        match ident {
            "package" => TokenKind::Package,
            "import" => TokenKind::Import,
            "var" => TokenKind::Var,
            "const" => TokenKind::Const,
            "type" => TokenKind::Type,
            "func" => TokenKind::Func,
            "interface" => TokenKind::Interface,
            "implements" => TokenKind::Implements,
            "struct" => TokenKind::Struct,
            "map" => TokenKind::Map,
            "chan" => TokenKind::Chan,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "range" => TokenKind::Range,
            "switch" => TokenKind::Switch,
            "case" => TokenKind::Case,
            "default" => TokenKind::Default,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "goto" => TokenKind::Goto,
            "fallthrough" => TokenKind::Fallthrough,
            "select" => TokenKind::Select,
            "go" => TokenKind::Go,
            "defer" => TokenKind::Defer,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            _ => TokenKind::Ident(ident.to_string()),
        }
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Token {
        // Skip whitespace first, which may set at_newline
        self.skip_whitespace();

        // Check for automatic semicolon insertion after whitespace
        if self.at_newline && self.should_insert_semi() {
            self.at_newline = false;
            self.prev_kind = Some(TokenKind::Semi);
            return Token::new(TokenKind::Semi, Span::point(self.pos));
        }

        self.at_newline = false;

        let start = self
            .pos
            .saturating_sub(self.ch.map(|c| c.len_utf8()).unwrap_or(0));

        let kind = match self.ch {
            None => TokenKind::Eof,
            Some(c) => match c {
                // Identifiers and keywords
                'a'..='z' | 'A'..='Z' | '_' => self.read_ident(),

                // Numbers
                '0'..='9' => self.read_number(),

                // Strings
                '"' => self.read_string(),

                // Operators and punctuation
                '+' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }
                '-' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }
                '*' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::StarAssign
                    } else {
                        TokenKind::Star
                    }
                }
                '/' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::SlashAssign
                    } else {
                        TokenKind::Slash
                    }
                }
                '%' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::PercentAssign
                    } else {
                        TokenKind::Percent
                    }
                }
                '=' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::Eq
                    } else {
                        TokenKind::Assign
                    }
                }
                '!' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Not
                    }
                }
                '<' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::LtEq
                    } else if self.ch == Some('-') {
                        self.read_char();
                        TokenKind::Arrow
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }
                '&' => {
                    self.read_char();
                    if self.ch == Some('&') {
                        self.read_char();
                        TokenKind::And
                    } else {
                        TokenKind::Invalid('&')
                    }
                }
                '|' => {
                    self.read_char();
                    if self.ch == Some('|') {
                        self.read_char();
                        TokenKind::Or
                    } else {
                        TokenKind::Invalid('|')
                    }
                }
                ':' => {
                    self.read_char();
                    if self.ch == Some('=') {
                        self.read_char();
                        TokenKind::ColonAssign
                    } else {
                        TokenKind::Colon
                    }
                }
                '.' => {
                    self.read_char();
                    if self.ch == Some('.') && self.peek_char() == Some('.') {
                        self.read_char();
                        self.read_char();
                        TokenKind::Ellipsis
                    } else {
                        TokenKind::Dot
                    }
                }
                '(' => {
                    self.read_char();
                    TokenKind::LParen
                }
                ')' => {
                    self.read_char();
                    TokenKind::RParen
                }
                '[' => {
                    self.read_char();
                    TokenKind::LBracket
                }
                ']' => {
                    self.read_char();
                    TokenKind::RBracket
                }
                '{' => {
                    self.read_char();
                    TokenKind::LBrace
                }
                '}' => {
                    self.read_char();
                    TokenKind::RBrace
                }
                ',' => {
                    self.read_char();
                    TokenKind::Comma
                }
                ';' => {
                    self.read_char();
                    TokenKind::Semi
                }
                _ => {
                    self.read_char();
                    TokenKind::Invalid(c)
                }
            },
        };

        // For ident/number/string, the end position is already at the correct place
        // after read_ident/read_number/read_string. For other tokens, self.pos is correct.
        let end = match self.ch {
            Some(c) => self.pos - c.len_utf8(),
            None => self.pos,
        };
        self.prev_kind = Some(kind.clone());
        Token::new(kind, Span::new(start, end))
    }

    /// Tokenize the entire input.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }
}

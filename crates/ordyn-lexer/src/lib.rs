use miette::Diagnostic;
use ordyn_diagnostics::Span;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug, Clone, PartialEq, Eq)]
pub enum LexError {
    #[error("Unexpected character: '{ch}'")]
    #[diagnostic(code(ordyn::lexer::unexpected_char))]
    UnexpectedChar {
        ch: char,
        #[label("unexpected character here")]
        span: Span,
    },

    #[error("Unterminated string literal")]
    #[diagnostic(code(ordyn::lexer::unterminated_string))]
    UnterminatedString {
        #[label("string started here but was never closed")]
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Entity,
    Capability,
    Contract,
    Flow,
    Boundary,
    Interface,
    Fn,
    Let,
    Mut,
    If,
    Else,
    While,
    Break,
    Continue,
    Match,
    Enum,
    Return,
    With,
    Requires,
    Ensures,
    Invariant,
    Pub,
    Priv,
    True,
    False,
    None,
    Old,
    Grant,
    Quota,
    Bind,

    // Identifiers & Literals
    Ident(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),

    // Symbols & Delimiters
    LParen,    // (
    RParen,    // )
    LBrace,    // {
    RBrace,    // }
    LBracket,  // [
    RBracket,  // ]
    Comma,     // ,
    Colon,     // :
    Semicolon, // ;
    Dot,       // .
    Arrow,     // ->
    FatArrow,  // =>

    // Operators
    Plus,    // +
    Minus,   // -
    Star,    // *
    Slash,   // /
    Percent, // %
    Eq,      // =
    EqEq,    // ==
    NotEq,   // !=
    Lt,      // <
    Lte,     // <=
    Gt,      // >
    Gte,     // >=
    AndAnd,  // &&
    OrOr,    // ||
    Bang,    // !

    // End of File
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            cursor: 0,
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments();
            if self.is_eof() {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span::new(self.cursor, 0),
                });
                break;
            }

            let start = self.cursor;
            let ch = self.peek_char();

            let kind = match ch {
                '(' => {
                    self.bump();
                    TokenKind::LParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RParen
                }
                '{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                '}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                '[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                ']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                ';' => {
                    self.bump();
                    TokenKind::Semicolon
                }
                ':' => {
                    self.bump();
                    TokenKind::Colon
                }
                '.' => {
                    self.bump();
                    TokenKind::Dot
                }
                '+' => {
                    self.bump();
                    TokenKind::Plus
                }
                '*' => {
                    self.bump();
                    TokenKind::Star
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '!' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '=' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::EqEq
                    } else if self.match_char('>') {
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Eq
                    }
                }
                '<' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::Lte
                    } else {
                        TokenKind::Lt
                    }
                }
                '>' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::Gte
                    } else {
                        TokenKind::Gt
                    }
                }
                '-' => {
                    self.bump();
                    if self.match_char('>') {
                        TokenKind::Arrow
                    } else {
                        TokenKind::Minus
                    }
                }
                '&' => {
                    self.bump();
                    if self.match_char('&') {
                        TokenKind::AndAnd
                    } else {
                        return Err(LexError::UnexpectedChar {
                            ch: '&',
                            span: Span::new(start, 1),
                        });
                    }
                }
                '|' => {
                    self.bump();
                    if self.match_char('|') {
                        TokenKind::OrOr
                    } else {
                        return Err(LexError::UnexpectedChar {
                            ch: '|',
                            span: Span::new(start, 1),
                        });
                    }
                }
                '"' => self.lex_string(start)?,
                c if c.is_ascii_digit() => self.lex_number(start),
                c if is_ident_start(c) => self.lex_ident(start),
                other => {
                    return Err(LexError::UnexpectedChar {
                        ch: other,
                        span: Span::new(start, 1),
                    });
                }
            };

            let length = self.cursor - start;
            tokens.push(Token {
                kind,
                span: Span::new(start, length),
            });
        }

        Ok(tokens)
    }

    fn peek_char(&self) -> char {
        self.source[self.cursor..].chars().next().unwrap_or('\0')
    }

    fn bump(&mut self) -> char {
        let ch = self.peek_char();
        self.cursor += ch.len_utf8();
        ch
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_eof() {
            return false;
        }
        if self.peek_char() == expected {
            self.bump();
            true
        } else {
            false
        }
    }

    fn is_eof(&self) -> bool {
        self.cursor >= self.bytes.len()
    }

    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_eof() {
            let ch = self.peek_char();
            if ch.is_whitespace() {
                self.bump();
            } else if ch == '/'
                && self.cursor + 1 < self.bytes.len()
                && self.bytes[self.cursor + 1] == b'/'
            {
                // Line comment
                while !self.is_eof() && self.peek_char() != '\n' {
                    self.bump();
                }
            } else if ch == '/'
                && self.cursor + 1 < self.bytes.len()
                && self.bytes[self.cursor + 1] == b'*'
            {
                // Block comment
                self.bump();
                self.bump();
                let mut depth = 1;
                while !self.is_eof() && depth > 0 {
                    if self.peek_char() == '*'
                        && self.cursor + 1 < self.bytes.len()
                        && self.bytes[self.cursor + 1] == b'/'
                    {
                        self.bump();
                        self.bump();
                        depth -= 1;
                    } else if self.peek_char() == '/'
                        && self.cursor + 1 < self.bytes.len()
                        && self.bytes[self.cursor + 1] == b'*'
                    {
                        self.bump();
                        self.bump();
                        depth += 1;
                    } else {
                        self.bump();
                    }
                }
            } else {
                break;
            }
        }
    }

    fn lex_string(&mut self, start: usize) -> Result<TokenKind, LexError> {
        self.bump(); // consume opening '"'
        let mut content = String::new();

        while !self.is_eof() {
            let ch = self.bump();
            if ch == '"' {
                return Ok(TokenKind::StringLiteral(content));
            } else if ch == '\\' {
                if self.is_eof() {
                    break;
                }
                match self.bump() {
                    'n' => content.push('\n'),
                    't' => content.push('\t'),
                    'r' => content.push('\r'),
                    '\\' => content.push('\\'),
                    '"' => content.push('"'),
                    other => content.push(other),
                }
            } else {
                content.push(ch);
            }
        }

        Err(LexError::UnterminatedString {
            span: Span::new(start, self.cursor - start),
        })
    }

    fn lex_number(&mut self, _start: usize) -> TokenKind {
        let num_start = self.cursor;
        while !self.is_eof() && self.peek_char().is_ascii_digit() {
            self.bump();
        }

        if !self.is_eof()
            && self.peek_char() == '.'
            && self.cursor + 1 < self.bytes.len()
            && (self.bytes[self.cursor + 1] as char).is_ascii_digit()
        {
            self.bump(); // consume '.'
            while !self.is_eof() && self.peek_char().is_ascii_digit() {
                self.bump();
            }
            let raw = &self.source[num_start..self.cursor];
            let val = raw.parse::<f64>().unwrap_or(0.0);
            TokenKind::FloatLiteral(val)
        } else {
            let raw = &self.source[num_start..self.cursor];
            let val = raw.parse::<i64>().unwrap_or(0);
            TokenKind::IntLiteral(val)
        }
    }

    fn lex_ident(&mut self, start: usize) -> TokenKind {
        while !self.is_eof() && is_ident_part(self.peek_char()) {
            self.bump();
        }

        let slice = &self.source[start..self.cursor];
        match slice {
            "entity" => TokenKind::Entity,
            "capability" => TokenKind::Capability,
            "contract" => TokenKind::Contract,
            "flow" => TokenKind::Flow,
            "boundary" => TokenKind::Boundary,
            "interface" => TokenKind::Interface,
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "match" => TokenKind::Match,
            "enum" => TokenKind::Enum,
            "return" => TokenKind::Return,
            "with" => TokenKind::With,
            "requires" => TokenKind::Requires,
            "ensures" => TokenKind::Ensures,
            "invariant" => TokenKind::Invariant,
            "pub" => TokenKind::Pub,
            "priv" => TokenKind::Priv,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "none" => TokenKind::None,
            "old" => TokenKind::Old,
            "grant" => TokenKind::Grant,
            "quota" => TokenKind::Quota,
            "bind" => TokenKind::Bind,
            ident => TokenKind::Ident(ident.to_string()),
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_part(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_basic_tokens() {
        let input = "entity Bank { pub fn deposit(amount: Int) -> Result; }";
        let lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("tokenization should succeed");
        assert_eq!(tokens[0].kind, TokenKind::Entity);
        assert_eq!(tokens[1].kind, TokenKind::Ident("Bank".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::LBrace);
        assert_eq!(tokens[3].kind, TokenKind::Pub);
        assert_eq!(tokens[4].kind, TokenKind::Fn);
    }

    #[test]
    fn test_lex_keywords_and_symbols() {
        let input = "capability NetConnect with [Net] requires ensures old";
        let lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Capability);
        assert_eq!(tokens[1].kind, TokenKind::Ident("NetConnect".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::With);
        assert_eq!(tokens[3].kind, TokenKind::LBracket);
        assert_eq!(tokens[4].kind, TokenKind::Ident("Net".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::RBracket);
        assert_eq!(tokens[6].kind, TokenKind::Requires);
        assert_eq!(tokens[7].kind, TokenKind::Ensures);
        assert_eq!(tokens[8].kind, TokenKind::Old);
    }

    #[test]
    fn test_unterminated_string_error() {
        let input = "let x = \"hello world";
        let lexer = Lexer::new(input);
        let result = lexer.tokenize();
        assert!(matches!(result, Err(LexError::UnterminatedString { .. })));
    }
}

//! CSS Tokenizer
//!
//! Tokenizes CSS input according to CSS Syntax Module Level 3.

use crate::error::{CssError, CssResult, SourceLocation};

/// CSS Token types
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Identifier (e.g., property names, keywords)
    Ident(String),
    /// Function token (identifier followed by '(')
    Function(String),
    /// At-keyword (e.g., @media, @import)
    AtKeyword(String),
    /// Hash token (e.g., #id, #fff)
    Hash(String, HashType),
    /// String token
    String(String),
    /// URL token
    Url(String),
    /// Number (without unit)
    Number(f32),
    /// Percentage
    Percentage(f32),
    /// Dimension (number with unit)
    Dimension(f32, String),
    /// Whitespace
    Whitespace,
    /// Colon ':'
    Colon,
    /// Semicolon ';'
    Semicolon,
    /// Comma ','
    Comma,
    /// Left bracket '['
    LeftBracket,
    /// Right bracket ']'
    RightBracket,
    /// Left paren '('
    LeftParen,
    /// Right paren ')'
    RightParen,
    /// Left brace '{'
    LeftBrace,
    /// Right brace '}'
    RightBrace,
    /// Delim (any other single character)
    Delim(char),
    /// End of file
    Eof,
}

/// Hash token type (id or unrestricted)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashType {
    /// Could be an ID selector
    Id,
    /// Unrestricted (e.g., color)
    Unrestricted,
}

/// CSS Tokenizer
pub struct Tokenizer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    position: usize,
    line: usize,
    column: usize,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.char_indices().peekable(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get the current source location
    pub fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.position)
    }

    /// Peek at the next character without consuming
    fn peek(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, c)| c)
    }

    /// Peek at the second character without consuming
    fn peek_second(&self) -> Option<char> {
        let mut iter = self.input[self.position..].chars();
        iter.next();
        iter.next()
    }

    /// Consume the next character
    fn advance(&mut self) -> Option<char> {
        if let Some((pos, c)) = self.chars.next() {
            self.position = pos + c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    /// Consume whitespace
    fn consume_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Consume a comment
    fn consume_comment(&mut self) -> bool {
        if self.peek() == Some('/') && self.peek_second() == Some('*') {
            self.advance(); // consume '/'
            self.advance(); // consume '*'

            loop {
                match self.advance() {
                    Some('*') if self.peek() == Some('/') => {
                        self.advance();
                        return true;
                    }
                    Some(_) => continue,
                    None => return true, // EOF in comment
                }
            }
        }
        false
    }

    /// Get the next token
    pub fn next_token(&mut self) -> CssResult<Token> {
        // Skip whitespace and comments, but track if we saw whitespace
        let mut saw_whitespace = false;
        loop {
            if let Some(c) = self.peek() {
                if c.is_ascii_whitespace() {
                    saw_whitespace = true;
                    self.consume_whitespace();
                } else if c == '/' && self.peek_second() == Some('*') {
                    self.consume_comment();
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if saw_whitespace {
            return Ok(Token::Whitespace);
        }

        let c = match self.peek() {
            Some(c) => c,
            None => return Ok(Token::Eof),
        };

        match c {
            ':' => {
                self.advance();
                Ok(Token::Colon)
            }
            ';' => {
                self.advance();
                Ok(Token::Semicolon)
            }
            ',' => {
                self.advance();
                Ok(Token::Comma)
            }
            '[' => {
                self.advance();
                Ok(Token::LeftBracket)
            }
            ']' => {
                self.advance();
                Ok(Token::RightBracket)
            }
            '(' => {
                self.advance();
                Ok(Token::LeftParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RightParen)
            }
            '{' => {
                self.advance();
                Ok(Token::LeftBrace)
            }
            '}' => {
                self.advance();
                Ok(Token::RightBrace)
            }
            '"' | '\'' => self.consume_string(),
            '#' => self.consume_hash(),
            '@' => self.consume_at_keyword(),
            '.' if self.peek_second().map(|c| c.is_ascii_digit()).unwrap_or(false) => {
                self.consume_number()
            }
            '0'..='9' => self.consume_number(),
            '+' | '-' => {
                if self.peek_second().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    self.consume_number()
                } else if self.peek_second() == Some('.') {
                    self.consume_number()
                } else if c == '-' && self.starts_identifier() {
                    self.consume_ident_like()
                } else {
                    self.advance();
                    Ok(Token::Delim(c))
                }
            }
            _ if is_ident_start(c) => self.consume_ident_like(),
            _ => {
                self.advance();
                Ok(Token::Delim(c))
            }
        }
    }

    /// Check if input starts an identifier
    fn starts_identifier(&self) -> bool {
        let mut chars = self.input[self.position..].chars();
        match chars.next() {
            Some('-') => match chars.next() {
                Some(c) if is_ident_start(c) => true,
                Some('-') => true,
                _ => false,
            },
            Some(c) if is_ident_start(c) => true,
            _ => false,
        }
    }

    /// Consume a string token
    fn consume_string(&mut self) -> CssResult<Token> {
        let quote = self.advance().unwrap();
        let mut value = String::new();

        loop {
            match self.advance() {
                Some(c) if c == quote => return Ok(Token::String(value)),
                Some('\\') => {
                    // Escape sequence
                    match self.peek() {
                        Some('\n') => {
                            self.advance();
                        }
                        Some(c) => {
                            self.advance();
                            value.push(c);
                        }
                        None => {}
                    }
                }
                Some('\n') => {
                    return Err(CssError::UnterminatedString { location: self.location() });
                }
                Some(c) => value.push(c),
                None => {
                    return Err(CssError::UnterminatedString { location: self.location() });
                }
            }
        }
    }

    /// Consume a hash token
    fn consume_hash(&mut self) -> CssResult<Token> {
        self.advance(); // consume '#'
        let mut value = String::new();
        let mut is_id = true;

        // First character determines if it could be an ID
        if let Some(c) = self.peek() {
            if is_ident_start(c) {
                is_id = true;
            } else if c.is_ascii_digit() {
                is_id = false;
            }
        }

        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let hash_type = if is_id && !value.is_empty() && is_ident_start(value.chars().next().unwrap()) {
            HashType::Id
        } else {
            HashType::Unrestricted
        };

        Ok(Token::Hash(value, hash_type))
    }

    /// Consume an at-keyword
    fn consume_at_keyword(&mut self) -> CssResult<Token> {
        self.advance(); // consume '@'
        let name = self.consume_ident_name();
        Ok(Token::AtKeyword(name))
    }

    /// Consume an identifier name
    fn consume_ident_name(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    /// Consume an identifier-like token (ident, function, or url)
    fn consume_ident_like(&mut self) -> CssResult<Token> {
        let name = self.consume_ident_name();

        // Check for function
        if self.peek() == Some('(') {
            self.advance(); // consume '('

            // Special handling for url()
            if name.eq_ignore_ascii_case("url") {
                // Skip whitespace
                self.consume_whitespace();

                // Check if it's a quoted string or bare URL
                match self.peek() {
                    Some('"') | Some('\'') => {
                        // Quoted URL - return as function, let parser handle it
                        return Ok(Token::Function(name));
                    }
                    Some(')') => {
                        self.advance();
                        return Ok(Token::Url(String::new()));
                    }
                    _ => {
                        // Bare URL
                        return self.consume_url();
                    }
                }
            }

            return Ok(Token::Function(name));
        }

        Ok(Token::Ident(name))
    }

    /// Consume a bare URL
    fn consume_url(&mut self) -> CssResult<Token> {
        let mut url = String::new();

        // Skip leading whitespace
        self.consume_whitespace();

        loop {
            match self.peek() {
                Some(')') => {
                    self.advance();
                    return Ok(Token::Url(url));
                }
                Some(c) if c.is_ascii_whitespace() => {
                    self.consume_whitespace();
                    if self.peek() == Some(')') {
                        self.advance();
                        return Ok(Token::Url(url));
                    }
                    // Whitespace in URL without closing paren is invalid
                    return Err(CssError::parse_error("Invalid URL", self.location()));
                }
                Some('\\') => {
                    self.advance();
                    if let Some(c) = self.advance() {
                        url.push(c);
                    }
                }
                Some(c) if c == '"' || c == '\'' || c == '(' => {
                    return Err(CssError::parse_error("Invalid character in URL", self.location()));
                }
                Some(c) => {
                    url.push(c);
                    self.advance();
                }
                None => {
                    return Err(CssError::parse_error("Unterminated URL", self.location()));
                }
            }
        }
    }

    /// Consume a number token
    fn consume_number(&mut self) -> CssResult<Token> {
        let mut num_str = String::new();

        // Optional sign
        if let Some(c) = self.peek() {
            if c == '+' || c == '-' {
                num_str.push(c);
                self.advance();
            }
        }

        // Integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if self.peek() == Some('.') {
            if let Some(c) = self.peek_second() {
                if c.is_ascii_digit() {
                    num_str.push('.');
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Exponent part
        if let Some(c) = self.peek() {
            if c == 'e' || c == 'E' {
                let mut exp = String::from(c);
                let saved_pos = self.position;

                self.advance();

                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        exp.push(sign);
                        self.advance();
                    }
                }

                if let Some(digit) = self.peek() {
                    if digit.is_ascii_digit() {
                        num_str.push_str(&exp);
                        while let Some(c) = self.peek() {
                            if c.is_ascii_digit() {
                                num_str.push(c);
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    } else {
                        // Not a valid exponent, restore position
                        // We need to rebuild the iterator from the start and skip to saved_pos
                        // to maintain correct absolute indices
                        self.position = saved_pos;
                        self.chars = self.input.char_indices().peekable();
                        while let Some(&(pos, _)) = self.chars.peek() {
                            if pos >= saved_pos {
                                break;
                            }
                            self.chars.next();
                        }
                    }
                } else {
                    // No digit after e/E and optional sign - not a valid exponent
                    self.position = saved_pos;
                    self.chars = self.input.char_indices().peekable();
                    while let Some(&(pos, _)) = self.chars.peek() {
                        if pos >= saved_pos {
                            break;
                        }
                        self.chars.next();
                    }
                }
            }
        }

        let value: f32 = num_str.parse().map_err(|_| {
            CssError::InvalidNumber {
                number: num_str.clone(),
                location: self.location(),
            }
        })?;

        // Check for percentage or unit
        if self.peek() == Some('%') {
            self.advance();
            return Ok(Token::Percentage(value));
        }

        // Check for dimension (unit)
        if let Some(c) = self.peek() {
            if is_ident_start(c) {
                let unit = self.consume_ident_name();
                return Ok(Token::Dimension(value, unit));
            }
        }

        Ok(Token::Number(value))
    }

    /// Tokenize all remaining input
    pub fn tokenize_all(&mut self) -> CssResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            if token == Token::Eof {
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}

/// Check if character can start an identifier
fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '-' || c > '\x7F'
}

/// Check if character can be part of an identifier
fn is_ident_char(c: char) -> bool {
    is_ident_start(c) || c.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<Token> {
        Tokenizer::new(input).tokenize_all().unwrap()
    }

    #[test]
    fn test_simple_rule() {
        let tokens = tokenize("p { color: red; }");
        assert!(matches!(tokens[0], Token::Ident(ref s) if s == "p"));
        assert!(matches!(tokens[2], Token::LeftBrace));
        assert!(matches!(tokens[4], Token::Ident(ref s) if s == "color"));
        assert!(matches!(tokens[5], Token::Colon));
        assert!(matches!(tokens[7], Token::Ident(ref s) if s == "red"));
        assert!(matches!(tokens[8], Token::Semicolon));
    }

    #[test]
    fn test_class_selector() {
        let tokens = tokenize(".container");
        assert!(matches!(tokens[0], Token::Delim('.')));
        assert!(matches!(tokens[1], Token::Ident(ref s) if s == "container"));
    }

    #[test]
    fn test_id_selector() {
        let tokens = tokenize("#main");
        assert!(matches!(tokens[0], Token::Hash(ref s, HashType::Id) if s == "main"));
    }

    #[test]
    fn test_color_hash() {
        let tokens = tokenize("#fff");
        assert!(matches!(tokens[0], Token::Hash(ref s, _) if s == "fff"));
    }

    #[test]
    fn test_hex_color_6() {
        let tokens = tokenize("#ff0000");
        assert!(matches!(tokens[0], Token::Hash(ref s, _) if s == "ff0000"));
    }

    #[test]
    fn test_number() {
        let tokens = tokenize("42");
        assert!(matches!(tokens[0], Token::Number(n) if (n - 42.0).abs() < 0.001));
    }

    #[test]
    fn test_float() {
        let tokens = tokenize("3.14");
        assert!(matches!(tokens[0], Token::Number(n) if (n - 3.14).abs() < 0.001));
    }

    #[test]
    fn test_percentage() {
        let tokens = tokenize("50%");
        assert!(matches!(tokens[0], Token::Percentage(n) if (n - 50.0).abs() < 0.001));
    }

    #[test]
    fn test_dimension_px() {
        let tokens = tokenize("100px");
        assert!(matches!(tokens[0], Token::Dimension(n, ref u) if (n - 100.0).abs() < 0.001 && u == "px"));
    }

    #[test]
    fn test_dimension_em() {
        let tokens = tokenize("1.5em");
        assert!(matches!(tokens[0], Token::Dimension(n, ref u) if (n - 1.5).abs() < 0.001 && u == "em"));
    }

    #[test]
    fn test_string_double() {
        let tokens = tokenize("\"hello world\"");
        assert!(matches!(tokens[0], Token::String(ref s) if s == "hello world"));
    }

    #[test]
    fn test_string_single() {
        let tokens = tokenize("'hello'");
        assert!(matches!(tokens[0], Token::String(ref s) if s == "hello"));
    }

    #[test]
    fn test_at_keyword() {
        let tokens = tokenize("@media");
        assert!(matches!(tokens[0], Token::AtKeyword(ref s) if s == "media"));
    }

    #[test]
    fn test_at_import() {
        let tokens = tokenize("@import");
        assert!(matches!(tokens[0], Token::AtKeyword(ref s) if s == "import"));
    }

    #[test]
    fn test_function() {
        let tokens = tokenize("rgb(255, 0, 0)");
        assert!(matches!(tokens[0], Token::Function(ref s) if s == "rgb"));
        assert!(matches!(tokens[1], Token::Number(n) if (n - 255.0).abs() < 0.001));
        assert!(matches!(tokens[2], Token::Comma));
    }

    #[test]
    fn test_url_bare() {
        let tokens = tokenize("url(http://example.com/image.png)");
        assert!(matches!(tokens[0], Token::Url(ref s) if s == "http://example.com/image.png"));
    }

    #[test]
    fn test_url_quoted() {
        let tokens = tokenize("url(\"http://example.com\")");
        assert!(matches!(tokens[0], Token::Function(ref s) if s == "url"));
        assert!(matches!(tokens[1], Token::String(ref s) if s == "http://example.com"));
    }

    #[test]
    fn test_combinators() {
        let tokens = tokenize("div > p + span ~ a");
        assert!(matches!(tokens[0], Token::Ident(ref s) if s == "div"));
        assert!(matches!(tokens[2], Token::Delim('>')));
        assert!(matches!(tokens[6], Token::Delim('+')));
        assert!(matches!(tokens[10], Token::Delim('~')));
    }

    #[test]
    fn test_attribute_selector() {
        let tokens = tokenize("[type=\"text\"]");
        assert!(matches!(tokens[0], Token::LeftBracket));
        assert!(matches!(tokens[1], Token::Ident(ref s) if s == "type"));
        assert!(matches!(tokens[2], Token::Delim('=')));
        assert!(matches!(tokens[3], Token::String(ref s) if s == "text"));
        assert!(matches!(tokens[4], Token::RightBracket));
    }

    #[test]
    fn test_pseudo_class() {
        let tokens = tokenize(":hover");
        assert!(matches!(tokens[0], Token::Colon));
        assert!(matches!(tokens[1], Token::Ident(ref s) if s == "hover"));
    }

    #[test]
    fn test_pseudo_element() {
        let tokens = tokenize("::before");
        assert!(matches!(tokens[0], Token::Colon));
        assert!(matches!(tokens[1], Token::Colon));
        assert!(matches!(tokens[2], Token::Ident(ref s) if s == "before"));
    }

    #[test]
    fn test_comment() {
        let tokens = tokenize("/* comment */ p");
        // Comment is skipped, but whitespace might be preserved
        let idents: Vec<_> = tokens.iter().filter(|t| matches!(t, Token::Ident(_))).collect();
        assert_eq!(idents.len(), 1);
    }

    #[test]
    fn test_negative_number() {
        let tokens = tokenize("-10px");
        assert!(matches!(tokens[0], Token::Dimension(n, ref u) if (n - (-10.0)).abs() < 0.001 && u == "px"));
    }

    #[test]
    fn test_multiple_declarations() {
        let tokens = tokenize("color: red; background: blue");
        let colons: Vec<_> = tokens.iter().filter(|t| matches!(t, Token::Colon)).collect();
        assert_eq!(colons.len(), 2);

        let semicolons: Vec<_> = tokens.iter().filter(|t| matches!(t, Token::Semicolon)).collect();
        assert_eq!(semicolons.len(), 1);
    }

    #[test]
    fn test_calc_function() {
        let tokens = tokenize("calc(100% - 20px)");
        assert!(matches!(tokens[0], Token::Function(ref s) if s == "calc"));
        assert!(matches!(tokens[1], Token::Percentage(n) if (n - 100.0).abs() < 0.001));
    }

    #[test]
    fn test_var_function() {
        let tokens = tokenize("var(--main-color)");
        assert!(matches!(tokens[0], Token::Function(ref s) if s == "var"));
        assert!(matches!(tokens[1], Token::Ident(ref s) if s == "--main-color"));
    }
}

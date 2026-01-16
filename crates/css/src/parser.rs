//! CSS Parser
//!
//! Parses complete CSS stylesheets, rules, and declarations.

use crate::error::{CssResult, SourceLocation};
use crate::tokenizer::{Token, Tokenizer};
use crate::selector::Selector;
use crate::value::{CssValue, ValueParser};

/// A CSS stylesheet
#[derive(Debug, Default)]
pub struct Stylesheet {
    /// All rules in the stylesheet
    pub rules: Vec<Rule>,
}

impl Stylesheet {
    /// Parse a CSS stylesheet
    pub fn parse(input: &str) -> CssResult<Self> {
        let mut parser = CssParser::new(input);
        parser.parse_stylesheet()
    }

    /// Create a new empty stylesheet
    pub fn new() -> Self {
        Self::default()
    }
}

/// A CSS rule
#[derive(Debug)]
pub enum Rule {
    /// Style rule (selector { declarations })
    Style(StyleRule),
    /// @import rule
    Import(ImportRule),
    /// @media rule
    Media(MediaRule),
    /// @font-face rule
    FontFace(FontFaceRule),
    /// @keyframes rule
    Keyframes(KeyframesRule),
}

/// A style rule (selector block)
#[derive(Debug)]
pub struct StyleRule {
    /// Selectors for this rule
    pub selectors: Vec<Selector>,
    /// Declarations
    pub declarations: Vec<Declaration>,
}

/// @import rule
#[derive(Debug)]
pub struct ImportRule {
    /// URL to import
    pub url: String,
    /// Media query (if any)
    pub media: Option<String>,
}

/// @media rule
#[derive(Debug)]
pub struct MediaRule {
    /// Media query
    pub query: String,
    /// Rules inside the media block
    pub rules: Vec<Rule>,
}

/// @font-face rule
#[derive(Debug)]
pub struct FontFaceRule {
    /// Declarations
    pub declarations: Vec<Declaration>,
}

/// @keyframes rule
#[derive(Debug)]
pub struct KeyframesRule {
    /// Animation name
    pub name: String,
    /// Keyframes
    pub keyframes: Vec<Keyframe>,
}

/// A single keyframe
#[derive(Debug)]
pub struct Keyframe {
    /// Selectors (e.g., "0%", "from", "50%, 75%")
    pub selectors: Vec<String>,
    /// Declarations
    pub declarations: Vec<Declaration>,
}

/// A CSS declaration (property: value)
#[derive(Debug, Clone)]
pub struct Declaration {
    /// Property name
    pub property: String,
    /// Property value
    pub value: CssValue,
    /// Whether !important was specified
    pub important: bool,
}

/// CSS Parser
pub struct CssParser<'a> {
    tokenizer: Tokenizer<'a>,
    current: Option<Token>,
}

impl<'a> CssParser<'a> {
    /// Create a new parser
    pub fn new(input: &'a str) -> Self {
        let mut tokenizer = Tokenizer::new(input);
        let current = tokenizer.next_token().ok();
        Self { tokenizer, current }
    }

    fn location(&self) -> SourceLocation {
        self.tokenizer.location()
    }

    fn advance(&mut self) -> CssResult<Option<Token>> {
        let prev = self.current.take();
        self.current = self.tokenizer.next_token().ok();
        Ok(prev)
    }

    fn peek(&self) -> Option<&Token> {
        self.current.as_ref()
    }

    fn skip_whitespace(&mut self) -> CssResult<()> {
        while let Some(Token::Whitespace) = self.peek() {
            self.advance()?;
        }
        Ok(())
    }

    /// Parse a complete stylesheet
    pub fn parse_stylesheet(&mut self) -> CssResult<Stylesheet> {
        let mut rules = Vec::new();

        loop {
            self.skip_whitespace()?;

            match self.peek() {
                None | Some(Token::Eof) => break,
                Some(Token::AtKeyword(_)) => {
                    if let Some(rule) = self.parse_at_rule()? {
                        rules.push(rule);
                    }
                }
                _ => {
                    if let Some(rule) = self.parse_style_rule()? {
                        rules.push(Rule::Style(rule));
                    }
                }
            }
        }

        Ok(Stylesheet { rules })
    }

    /// Parse an at-rule
    fn parse_at_rule(&mut self) -> CssResult<Option<Rule>> {
        let name = match self.advance()? {
            Some(Token::AtKeyword(name)) => name.to_ascii_lowercase(),
            _ => return Ok(None),
        };

        self.skip_whitespace()?;

        match name.as_str() {
            "import" => self.parse_import_rule(),
            "media" => self.parse_media_rule(),
            "font-face" => self.parse_font_face_rule(),
            "keyframes" | "-webkit-keyframes" => self.parse_keyframes_rule(),
            _ => {
                // Skip unknown at-rules
                self.skip_until_semicolon_or_block()?;
                Ok(None)
            }
        }
    }

    /// Parse @import rule
    fn parse_import_rule(&mut self) -> CssResult<Option<Rule>> {
        self.skip_whitespace()?;

        let url = match self.advance()? {
            Some(Token::String(s)) => s,
            Some(Token::Url(u)) => u,
            Some(Token::Function(name)) if name.eq_ignore_ascii_case("url") => {
                // Parse url() function
                self.skip_whitespace()?;
                let url = match self.advance()? {
                    Some(Token::String(s)) => s,
                    Some(Token::Ident(s)) => s,
                    _ => return Ok(None),
                };
                self.skip_whitespace()?;
                // Consume ')'
                if matches!(self.peek(), Some(Token::RightParen)) {
                    self.advance()?;
                }
                url
            }
            _ => return Ok(None),
        };

        self.skip_whitespace()?;

        // Optional media query
        let media = self.collect_until_semicolon()?;
        let media = if media.is_empty() { None } else { Some(media) };

        // Consume semicolon
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance()?;
        }

        Ok(Some(Rule::Import(ImportRule { url, media })))
    }

    /// Parse @media rule
    fn parse_media_rule(&mut self) -> CssResult<Option<Rule>> {
        self.skip_whitespace()?;

        let query = self.collect_until_brace()?;

        // Consume '{'
        if !matches!(self.peek(), Some(Token::LeftBrace)) {
            return Ok(None);
        }
        self.advance()?;

        // Parse nested rules
        let mut rules = Vec::new();
        loop {
            self.skip_whitespace()?;

            match self.peek() {
                None | Some(Token::Eof) | Some(Token::RightBrace) => break,
                Some(Token::AtKeyword(_)) => {
                    if let Some(rule) = self.parse_at_rule()? {
                        rules.push(rule);
                    }
                }
                _ => {
                    if let Some(rule) = self.parse_style_rule()? {
                        rules.push(Rule::Style(rule));
                    }
                }
            }
        }

        // Consume '}'
        if matches!(self.peek(), Some(Token::RightBrace)) {
            self.advance()?;
        }

        Ok(Some(Rule::Media(MediaRule { query, rules })))
    }

    /// Parse @font-face rule
    fn parse_font_face_rule(&mut self) -> CssResult<Option<Rule>> {
        self.skip_whitespace()?;

        // Consume '{'
        if !matches!(self.peek(), Some(Token::LeftBrace)) {
            return Ok(None);
        }
        self.advance()?;

        let declarations = self.parse_declaration_block()?;

        Ok(Some(Rule::FontFace(FontFaceRule { declarations })))
    }

    /// Parse @keyframes rule
    fn parse_keyframes_rule(&mut self) -> CssResult<Option<Rule>> {
        self.skip_whitespace()?;

        let name = match self.advance()? {
            Some(Token::Ident(name)) => name,
            Some(Token::String(name)) => name,
            _ => return Ok(None),
        };

        self.skip_whitespace()?;

        // Consume '{'
        if !matches!(self.peek(), Some(Token::LeftBrace)) {
            return Ok(None);
        }
        self.advance()?;

        let mut keyframes = Vec::new();

        loop {
            self.skip_whitespace()?;

            if matches!(self.peek(), None | Some(Token::Eof) | Some(Token::RightBrace)) {
                break;
            }

            if let Some(keyframe) = self.parse_keyframe()? {
                keyframes.push(keyframe);
            }
        }

        // Consume '}'
        if matches!(self.peek(), Some(Token::RightBrace)) {
            self.advance()?;
        }

        Ok(Some(Rule::Keyframes(KeyframesRule { name, keyframes })))
    }

    /// Parse a single keyframe
    fn parse_keyframe(&mut self) -> CssResult<Option<Keyframe>> {
        let selectors = self.collect_keyframe_selectors()?;

        if selectors.is_empty() {
            return Ok(None);
        }

        // Consume '{'
        if !matches!(self.peek(), Some(Token::LeftBrace)) {
            return Ok(None);
        }
        self.advance()?;

        let declarations = self.parse_declaration_block()?;

        Ok(Some(Keyframe { selectors, declarations }))
    }

    /// Collect keyframe selectors (from, to, percentages)
    fn collect_keyframe_selectors(&mut self) -> CssResult<Vec<String>> {
        let mut selectors = Vec::new();
        let mut current = String::new();

        loop {
            self.skip_whitespace()?;

            match self.peek().cloned() {
                Some(Token::LeftBrace) | None | Some(Token::Eof) => break,
                Some(Token::Comma) => {
                    self.advance()?;
                    if !current.is_empty() {
                        selectors.push(current.trim().to_string());
                        current = String::new();
                    }
                }
                Some(Token::Ident(s)) => {
                    self.advance()?;
                    current.push_str(&s);
                }
                Some(Token::Percentage(n)) => {
                    self.advance()?;
                    current.push_str(&format!("{}%", n));
                }
                Some(Token::Number(n)) => {
                    self.advance()?;
                    current.push_str(&format!("{}", n));
                }
                _ => {
                    self.advance()?;
                }
            }
        }

        if !current.is_empty() {
            selectors.push(current.trim().to_string());
        }

        Ok(selectors)
    }

    /// Parse a style rule (selectors { declarations })
    fn parse_style_rule(&mut self) -> CssResult<Option<StyleRule>> {
        // Collect selector text
        let selector_text = self.collect_until_brace()?;

        if selector_text.is_empty() {
            return Ok(None);
        }

        // Parse selectors
        let selectors = Selector::parse_list(&selector_text)?;

        if selectors.is_empty() {
            return Ok(None);
        }

        // Consume '{'
        if !matches!(self.peek(), Some(Token::LeftBrace)) {
            return Ok(None);
        }
        self.advance()?;

        // Parse declarations
        let declarations = self.parse_declaration_block()?;

        Ok(Some(StyleRule { selectors, declarations }))
    }

    /// Parse a declaration block (inside { })
    fn parse_declaration_block(&mut self) -> CssResult<Vec<Declaration>> {
        let mut declarations = Vec::new();

        loop {
            self.skip_whitespace()?;

            match self.peek() {
                None | Some(Token::Eof) | Some(Token::RightBrace) => break,
                _ => {
                    if let Some(decl) = self.parse_declaration()? {
                        declarations.push(decl);
                    }
                }
            }
        }

        // Consume '}'
        if matches!(self.peek(), Some(Token::RightBrace)) {
            self.advance()?;
        }

        Ok(declarations)
    }

    /// Parse a single declaration
    fn parse_declaration(&mut self) -> CssResult<Option<Declaration>> {
        self.skip_whitespace()?;

        // Get property name
        let property = match self.peek().cloned() {
            Some(Token::Ident(name)) => {
                self.advance()?;
                name.to_ascii_lowercase()
            }
            _ => {
                // Skip to next semicolon or brace
                self.skip_until_semicolon_or_brace()?;
                return Ok(None);
            }
        };

        self.skip_whitespace()?;

        // Expect colon
        if !matches!(self.peek(), Some(Token::Colon)) {
            self.skip_until_semicolon_or_brace()?;
            return Ok(None);
        }
        self.advance()?;

        self.skip_whitespace()?;

        // Parse value
        let (value, important) = self.parse_declaration_value()?;

        // Consume semicolon if present
        self.skip_whitespace()?;
        if matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance()?;
        }

        Ok(Some(Declaration { property, value, important }))
    }

    /// Parse a declaration value
    fn parse_declaration_value(&mut self) -> CssResult<(CssValue, bool)> {
        let mut values = Vec::new();
        let mut important = false;
        let location = self.location();

        loop {
            self.skip_whitespace()?;

            match self.peek().cloned() {
                None | Some(Token::Eof) | Some(Token::Semicolon) | Some(Token::RightBrace) => break,
                Some(Token::Delim('!')) => {
                    self.advance()?;
                    self.skip_whitespace()?;
                    if let Some(Token::Ident(s)) = self.peek() {
                        if s.eq_ignore_ascii_case("important") {
                            important = true;
                            self.advance()?;
                        }
                    }
                }
                Some(Token::Function(name)) => {
                    self.advance()?;
                    let func_value = self.parse_function_value(&name, location)?;
                    values.push(func_value);
                }
                Some(token) => {
                    self.advance()?;
                    // Skip commas for now (used in list values)
                    if matches!(token, Token::Comma) {
                        continue;
                    }
                    if let Ok(value) = ValueParser::parse_token(&token, location) {
                        values.push(value);
                    }
                }
            }
        }

        let value = if values.is_empty() {
            CssValue::Keyword("initial".to_string())
        } else if values.len() == 1 {
            values.remove(0)
        } else {
            CssValue::List(values)
        };

        Ok((value, important))
    }

    /// Parse a function value (rgb, calc, var, etc.)
    fn parse_function_value(&mut self, name: &str, location: SourceLocation) -> CssResult<CssValue> {
        let mut args = Vec::new();
        let mut paren_depth = 1;

        loop {
            match self.peek().cloned() {
                Some(Token::LeftParen) => {
                    paren_depth += 1;
                    self.advance()?;
                }
                Some(Token::RightParen) => {
                    paren_depth -= 1;
                    self.advance()?;
                    if paren_depth == 0 {
                        break;
                    }
                }
                Some(Token::Eof) | None => break,
                Some(token) => {
                    self.advance()?;
                    args.push(token);
                }
            }
        }

        // Handle specific functions
        match name.to_ascii_lowercase().as_str() {
            "rgb" | "rgba" => {
                let color = ValueParser::parse_rgb(&args, location)?;
                Ok(CssValue::Color(color))
            }
            "hsl" | "hsla" => {
                let color = ValueParser::parse_hsl(&args, location)?;
                Ok(CssValue::Color(color))
            }
            "url" => {
                // Extract URL from args
                for arg in args {
                    if let Token::String(url) = arg {
                        return Ok(CssValue::Url(url));
                    }
                }
                Ok(CssValue::Url(String::new()))
            }
            _ => {
                // Generic function - convert args to values
                let mut arg_values = Vec::new();
                for arg in args {
                    if !matches!(arg, Token::Whitespace | Token::Comma) {
                        if let Ok(v) = ValueParser::parse_token(&arg, location) {
                            arg_values.push(v);
                        }
                    }
                }
                Ok(CssValue::Function(name.to_string(), arg_values))
            }
        }
    }

    /// Collect tokens until a left brace, returning as string
    fn collect_until_brace(&mut self) -> CssResult<String> {
        let mut text = String::new();

        loop {
            match self.peek() {
                None | Some(Token::Eof) | Some(Token::LeftBrace) => break,
                _ => {
                    if let Some(token) = self.advance()? {
                        text.push_str(&token_to_string(&token));
                    }
                }
            }
        }

        Ok(text.trim().to_string())
    }

    /// Collect tokens until a semicolon, returning as string
    fn collect_until_semicolon(&mut self) -> CssResult<String> {
        let mut text = String::new();

        loop {
            match self.peek() {
                None | Some(Token::Eof) | Some(Token::Semicolon) => break,
                _ => {
                    if let Some(token) = self.advance()? {
                        text.push_str(&token_to_string(&token));
                    }
                }
            }
        }

        Ok(text.trim().to_string())
    }

    /// Skip tokens until semicolon or brace
    fn skip_until_semicolon_or_brace(&mut self) -> CssResult<()> {
        loop {
            match self.peek() {
                None | Some(Token::Eof) | Some(Token::Semicolon) | Some(Token::LeftBrace) | Some(Token::RightBrace) => {
                    if matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance()?;
                    }
                    break;
                }
                _ => {
                    self.advance()?;
                }
            }
        }
        Ok(())
    }

    /// Skip tokens until semicolon or end of block
    fn skip_until_semicolon_or_block(&mut self) -> CssResult<()> {
        let mut brace_depth = 0;

        loop {
            match self.peek() {
                None | Some(Token::Eof) => break,
                Some(Token::Semicolon) if brace_depth == 0 => {
                    self.advance()?;
                    break;
                }
                Some(Token::LeftBrace) => {
                    brace_depth += 1;
                    self.advance()?;
                }
                Some(Token::RightBrace) => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth -= 1;
                    self.advance()?;
                    if brace_depth == 0 {
                        break;
                    }
                }
                _ => {
                    self.advance()?;
                }
            }
        }

        Ok(())
    }
}

/// Convert a token to its string representation (for collecting selector text)
fn token_to_string(token: &Token) -> String {
    match token {
        Token::Ident(s) => s.clone(),
        Token::Function(s) => format!("{}(", s),
        Token::AtKeyword(s) => format!("@{}", s),
        Token::Hash(s, _) => format!("#{}", s),
        Token::String(s) => format!("\"{}\"", s),
        Token::Url(s) => format!("url({})", s),
        Token::Number(n) => n.to_string(),
        Token::Percentage(n) => format!("{}%", n),
        Token::Dimension(n, u) => format!("{}{}", n, u),
        Token::Whitespace => " ".to_string(),
        Token::Colon => ":".to_string(),
        Token::Semicolon => ";".to_string(),
        Token::Comma => ",".to_string(),
        Token::LeftBracket => "[".to_string(),
        Token::RightBracket => "]".to_string(),
        Token::LeftParen => "(".to_string(),
        Token::RightParen => ")".to_string(),
        Token::LeftBrace => "{".to_string(),
        Token::RightBrace => "}".to_string(),
        Token::Delim(c) => c.to_string(),
        Token::Eof => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_rule() {
        let css = "p { color: red; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        if let Rule::Style(rule) = &stylesheet.rules[0] {
            assert_eq!(rule.selectors.len(), 1);
            assert_eq!(rule.declarations.len(), 1);
            assert_eq!(rule.declarations[0].property, "color");
        } else {
            panic!("Expected style rule");
        }
    }

    #[test]
    fn test_multiple_declarations() {
        let css = "p { color: red; font-size: 16px; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            assert_eq!(rule.declarations.len(), 2);
            assert_eq!(rule.declarations[0].property, "color");
            assert_eq!(rule.declarations[1].property, "font-size");
        }
    }

    #[test]
    fn test_hex_color() {
        let css = "p { color: #ff0000; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::Color(color) = &rule.declarations[0].value {
                assert_eq!(color.r, 255);
                assert_eq!(color.g, 0);
                assert_eq!(color.b, 0);
            } else {
                panic!("Expected color value");
            }
        }
    }

    #[test]
    fn test_rgb_function() {
        let css = "p { color: rgb(255, 128, 0); }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::Color(color) = &rule.declarations[0].value {
                assert_eq!(color.r, 255);
                assert_eq!(color.g, 128);
                assert_eq!(color.b, 0);
            } else {
                panic!("Expected color value");
            }
        }
    }

    #[test]
    fn test_important() {
        let css = "p { color: red !important; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            assert!(rule.declarations[0].important);
        }
    }

    #[test]
    fn test_multiple_selectors() {
        let css = "h1, h2, h3 { color: blue; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            assert_eq!(rule.selectors.len(), 3);
        }
    }

    #[test]
    fn test_complex_selector() {
        let css = "div.container > p.intro { font-size: 18px; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            assert_eq!(rule.selectors.len(), 1);
            assert!(!rule.selectors[0].parts.is_empty());
        }
    }

    #[test]
    fn test_media_rule() {
        let css = "@media screen and (max-width: 600px) { p { font-size: 14px; } }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        if let Rule::Media(media) = &stylesheet.rules[0] {
            assert!(media.query.contains("screen"));
            assert_eq!(media.rules.len(), 1);
        } else {
            panic!("Expected media rule");
        }
    }

    #[test]
    fn test_import_rule() {
        let css = "@import url('styles.css');";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Import(import) = &stylesheet.rules[0] {
            assert_eq!(import.url, "styles.css");
        } else {
            panic!("Expected import rule");
        }
    }

    #[test]
    fn test_import_string() {
        let css = "@import \"styles.css\";";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Import(import) = &stylesheet.rules[0] {
            assert_eq!(import.url, "styles.css");
        }
    }

    #[test]
    fn test_font_face() {
        let css = "@font-face { font-family: 'MyFont'; src: url('myfont.woff2'); }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::FontFace(ff) = &stylesheet.rules[0] {
            assert!(!ff.declarations.is_empty());
        } else {
            panic!("Expected font-face rule");
        }
    }

    #[test]
    fn test_keyframes() {
        let css = "@keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Keyframes(kf) = &stylesheet.rules[0] {
            assert_eq!(kf.name, "fadeIn");
            assert_eq!(kf.keyframes.len(), 2);
        } else {
            panic!("Expected keyframes rule");
        }
    }

    #[test]
    fn test_percentage_keyframes() {
        let css = "@keyframes slide { 0% { left: 0; } 50% { left: 50px; } 100% { left: 100px; } }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Keyframes(kf) = &stylesheet.rules[0] {
            assert_eq!(kf.keyframes.len(), 3);
        }
    }

    #[test]
    fn test_dimension_values() {
        let css = "p { margin: 10px 20em 5rem 15%; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::List(values) = &rule.declarations[0].value {
                assert_eq!(values.len(), 4);
            }
        }
    }

    #[test]
    fn test_var_function() {
        let css = "p { color: var(--main-color); }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::Function(name, _) = &rule.declarations[0].value {
                assert_eq!(name, "var");
            } else {
                panic!("Expected function value");
            }
        }
    }

    #[test]
    fn test_multiple_rules() {
        let css = "p { color: red; } div { color: blue; } span { color: green; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        assert_eq!(stylesheet.rules.len(), 3);
    }

    #[test]
    fn test_empty_stylesheet() {
        let css = "   ";
        let stylesheet = Stylesheet::parse(css).unwrap();

        assert!(stylesheet.rules.is_empty());
    }

    #[test]
    fn test_comment_ignored() {
        let css = "/* comment */ p { color: red; }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
    }

    #[test]
    fn test_url_value() {
        let css = "div { background: url('image.png'); }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::Url(url) = &rule.declarations[0].value {
                assert_eq!(url, "image.png");
            } else {
                panic!("Expected URL value");
            }
        }
    }

    #[test]
    fn test_hsl_color() {
        let css = "p { color: hsl(120, 100%, 50%); }";
        let stylesheet = Stylesheet::parse(css).unwrap();

        if let Rule::Style(rule) = &stylesheet.rules[0] {
            if let CssValue::Color(color) = &rule.declarations[0].value {
                // Pure green in HSL
                assert!(color.g > color.r);
                assert!(color.g > color.b);
            } else {
                panic!("Expected color value");
            }
        }
    }
}

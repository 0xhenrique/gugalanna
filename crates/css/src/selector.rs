//! CSS Selector Parser
//!
//! Parses CSS selectors according to Selectors Level 4.

use crate::error::{CssError, CssResult, SourceLocation};
use crate::tokenizer::{Token, HashType, Tokenizer};

/// A complete selector (may contain multiple compound selectors)
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    /// The compound selectors and combinators that make up this selector
    pub parts: Vec<SelectorPart>,
    /// Specificity of this selector (a, b, c)
    pub specificity: Specificity,
}

/// A part of a compound selector
#[derive(Debug, Clone, PartialEq)]
pub enum SelectorPart {
    /// Universal selector (*)
    Universal,
    /// Type selector (e.g., div, p, span)
    Type(String),
    /// Class selector (e.g., .container)
    Class(String),
    /// ID selector (e.g., #main)
    Id(String),
    /// Attribute selector (e.g., [type="text"])
    Attribute {
        name: String,
        op: Option<AttributeOp>,
        value: Option<String>,
        case_insensitive: bool,
    },
    /// Pseudo-class (e.g., :hover, :nth-child(2n))
    PseudoClass {
        name: String,
        args: Option<String>,
    },
    /// Pseudo-element (e.g., ::before, ::after)
    PseudoElement {
        name: String,
        args: Option<String>,
    },
    /// Combinator between compound selectors
    Combinator(Combinator),
}

/// Attribute selector operators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeOp {
    /// [attr=value] - exact match
    Equals,
    /// [attr~=value] - contains word
    Includes,
    /// [attr|=value] - starts with value or value-
    DashMatch,
    /// [attr^=value] - starts with
    PrefixMatch,
    /// [attr$=value] - ends with
    SuffixMatch,
    /// [attr*=value] - contains
    SubstringMatch,
}

/// Selector combinators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Descendant combinator (space)
    Descendant,
    /// Child combinator (>)
    Child,
    /// Next sibling combinator (+)
    NextSibling,
    /// Subsequent sibling combinator (~)
    SubsequentSibling,
}

/// Selector specificity (a, b, c)
/// a = ID selectors
/// b = class selectors, attribute selectors, pseudo-classes
/// c = type selectors, pseudo-elements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Specificity {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

impl Specificity {
    pub fn new(a: u32, b: u32, c: u32) -> Self {
        Self { a, b, c }
    }

    /// Compare specificities
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        self.a.cmp(&other.a)
            .then_with(|| self.b.cmp(&other.b))
            .then_with(|| self.c.cmp(&other.c))
    }

    /// Add another specificity
    pub fn add(&mut self, other: &Self) {
        self.a += other.a;
        self.b += other.b;
        self.c += other.c;
    }
}

impl Ord for Specificity {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare(other)
    }
}

impl PartialOrd for Specificity {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Selector {
    /// Parse a selector string
    pub fn parse(input: &str) -> CssResult<Self> {
        let mut parser = SelectorParser::new(input);
        parser.parse_selector()
    }

    /// Parse a comma-separated list of selectors
    pub fn parse_list(input: &str) -> CssResult<Vec<Self>> {
        let mut parser = SelectorParser::new(input);
        parser.parse_selector_list()
    }
}

/// Selector parser
struct SelectorParser<'a> {
    tokenizer: Tokenizer<'a>,
    current: Option<Token>,
}

impl<'a> SelectorParser<'a> {
    fn new(input: &'a str) -> Self {
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

    fn parse_selector_list(&mut self) -> CssResult<Vec<Selector>> {
        let mut selectors = Vec::new();

        loop {
            self.skip_whitespace()?;

            if matches!(self.peek(), None | Some(Token::Eof)) {
                break;
            }

            selectors.push(self.parse_selector()?);

            self.skip_whitespace()?;

            match self.peek() {
                Some(Token::Comma) => {
                    self.advance()?;
                }
                _ => break,
            }
        }

        Ok(selectors)
    }

    #[allow(unused_assignments)]
    fn parse_selector(&mut self) -> CssResult<Selector> {
        let mut parts = Vec::new();
        let mut specificity = Specificity::default();
        let mut saw_simple_selector = false;

        loop {
            // Track if there's whitespace before the next token
            let had_whitespace = matches!(self.peek(), Some(Token::Whitespace));
            self.skip_whitespace()?;

            match self.peek() {
                None | Some(Token::Eof) | Some(Token::Comma) | Some(Token::LeftBrace) => break,
                _ => {}
            }

            // Check for explicit combinator (>, +, ~)
            if let Some(comb) = self.try_parse_combinator()? {
                if saw_simple_selector {
                    parts.push(SelectorPart::Combinator(comb));
                    saw_simple_selector = false;
                    continue;
                }
            } else if saw_simple_selector && had_whitespace {
                // Whitespace between compound selectors = descendant combinator
                parts.push(SelectorPart::Combinator(Combinator::Descendant));
                saw_simple_selector = false;
            }

            // Parse simple selector
            if let Some((part, spec)) = self.try_parse_simple_selector()? {
                parts.push(part);
                specificity.add(&spec);
                saw_simple_selector = true;
            } else {
                break;
            }
        }

        if parts.is_empty() {
            return Err(CssError::InvalidSelector {
                selector: String::new(),
                location: self.location(),
            });
        }

        Ok(Selector { parts, specificity })
    }

    fn try_parse_combinator(&mut self) -> CssResult<Option<Combinator>> {
        match self.peek() {
            Some(Token::Delim('>')) => {
                self.advance()?;
                Ok(Some(Combinator::Child))
            }
            Some(Token::Delim('+')) => {
                self.advance()?;
                Ok(Some(Combinator::NextSibling))
            }
            Some(Token::Delim('~')) => {
                self.advance()?;
                Ok(Some(Combinator::SubsequentSibling))
            }
            _ => Ok(None),
        }
    }

    fn try_parse_simple_selector(&mut self) -> CssResult<Option<(SelectorPart, Specificity)>> {
        match self.peek().cloned() {
            Some(Token::Delim('*')) => {
                self.advance()?;
                Ok(Some((SelectorPart::Universal, Specificity::default())))
            }
            Some(Token::Ident(name)) => {
                self.advance()?;
                Ok(Some((SelectorPart::Type(name.to_ascii_lowercase()), Specificity::new(0, 0, 1))))
            }
            Some(Token::Hash(name, HashType::Id)) => {
                self.advance()?;
                Ok(Some((SelectorPart::Id(name), Specificity::new(1, 0, 0))))
            }
            Some(Token::Hash(name, HashType::Unrestricted)) => {
                // Still treat as ID selector
                self.advance()?;
                Ok(Some((SelectorPart::Id(name), Specificity::new(1, 0, 0))))
            }
            Some(Token::Delim('.')) => {
                self.advance()?;
                if let Some(Token::Ident(name)) = self.advance()? {
                    Ok(Some((SelectorPart::Class(name), Specificity::new(0, 1, 0))))
                } else {
                    Err(CssError::InvalidSelector {
                        selector: ".".to_string(),
                        location: self.location(),
                    })
                }
            }
            Some(Token::LeftBracket) => {
                self.parse_attribute_selector()
            }
            Some(Token::Colon) => {
                self.parse_pseudo_selector()
            }
            _ => Ok(None),
        }
    }

    fn parse_attribute_selector(&mut self) -> CssResult<Option<(SelectorPart, Specificity)>> {
        self.advance()?; // consume '['
        self.skip_whitespace()?;

        // Get attribute name
        let name = match self.advance()? {
            Some(Token::Ident(name)) => name,
            _ => return Err(CssError::InvalidSelector {
                selector: "[".to_string(),
                location: self.location(),
            }),
        };

        self.skip_whitespace()?;

        // Check for operator
        let op = match self.peek() {
            Some(Token::Delim('=')) => {
                self.advance()?;
                Some(AttributeOp::Equals)
            }
            Some(Token::Delim('~')) => {
                self.advance()?;
                if matches!(self.peek(), Some(Token::Delim('='))) {
                    self.advance()?;
                    Some(AttributeOp::Includes)
                } else {
                    return Err(CssError::InvalidSelector {
                        selector: format!("[{}~", name),
                        location: self.location(),
                    });
                }
            }
            Some(Token::Delim('|')) => {
                self.advance()?;
                if matches!(self.peek(), Some(Token::Delim('='))) {
                    self.advance()?;
                    Some(AttributeOp::DashMatch)
                } else {
                    return Err(CssError::InvalidSelector {
                        selector: format!("[{}|", name),
                        location: self.location(),
                    });
                }
            }
            Some(Token::Delim('^')) => {
                self.advance()?;
                if matches!(self.peek(), Some(Token::Delim('='))) {
                    self.advance()?;
                    Some(AttributeOp::PrefixMatch)
                } else {
                    return Err(CssError::InvalidSelector {
                        selector: format!("[{}^", name),
                        location: self.location(),
                    });
                }
            }
            Some(Token::Delim('$')) => {
                self.advance()?;
                if matches!(self.peek(), Some(Token::Delim('='))) {
                    self.advance()?;
                    Some(AttributeOp::SuffixMatch)
                } else {
                    return Err(CssError::InvalidSelector {
                        selector: format!("[{}$", name),
                        location: self.location(),
                    });
                }
            }
            Some(Token::Delim('*')) => {
                self.advance()?;
                if matches!(self.peek(), Some(Token::Delim('='))) {
                    self.advance()?;
                    Some(AttributeOp::SubstringMatch)
                } else {
                    return Err(CssError::InvalidSelector {
                        selector: format!("[{}*", name),
                        location: self.location(),
                    });
                }
            }
            _ => None,
        };

        self.skip_whitespace()?;

        // Get value if operator exists
        let value = if op.is_some() {
            self.skip_whitespace()?;
            match self.advance()? {
                Some(Token::Ident(v)) => Some(v),
                Some(Token::String(v)) => Some(v),
                _ => return Err(CssError::InvalidSelector {
                    selector: format!("[{}=", name),
                    location: self.location(),
                }),
            }
        } else {
            None
        };

        self.skip_whitespace()?;

        // Check for case insensitivity flag
        let case_insensitive = if let Some(Token::Ident(flag)) = self.peek() {
            if flag.eq_ignore_ascii_case("i") || flag.eq_ignore_ascii_case("s") {
                let is_insensitive = flag.eq_ignore_ascii_case("i");
                self.advance()?;
                is_insensitive
            } else {
                false
            }
        } else {
            false
        };

        self.skip_whitespace()?;

        // Consume ']'
        match self.advance()? {
            Some(Token::RightBracket) => {}
            _ => return Err(CssError::InvalidSelector {
                selector: format!("[{}", name),
                location: self.location(),
            }),
        }

        Ok(Some((
            SelectorPart::Attribute { name, op, value, case_insensitive },
            Specificity::new(0, 1, 0),
        )))
    }

    fn parse_pseudo_selector(&mut self) -> CssResult<Option<(SelectorPart, Specificity)>> {
        self.advance()?; // consume first ':'

        // Check for pseudo-element (::)
        let is_element = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance()?;
            true
        } else {
            false
        };

        // Get name - handle both Ident and Function tokens
        let (name, is_function) = match self.advance()? {
            Some(Token::Ident(name)) => (name.to_ascii_lowercase(), false),
            Some(Token::Function(name)) => (name.to_ascii_lowercase(), true),
            _ => return Err(CssError::InvalidSelector {
                selector: ":".to_string(),
                location: self.location(),
            }),
        };

        // Parse arguments if it's a functional pseudo-class/element
        // For Function tokens, the '(' was already consumed by the tokenizer
        let args = if is_function {
            // Function token means '(' was already consumed, parse until ')'
            let args = self.parse_pseudo_args()?;
            Some(args)
        } else if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance()?; // consume '('
            let args = self.parse_pseudo_args()?;
            Some(args)
        } else {
            None
        };

        // Legacy pseudo-elements with single colon
        let is_element = is_element || matches!(name.as_str(), "before" | "after" | "first-line" | "first-letter");

        if is_element {
            Ok(Some((
                SelectorPart::PseudoElement { name, args },
                Specificity::new(0, 0, 1),
            )))
        } else {
            // :not() and :is() have special specificity rules
            let specificity = match name.as_str() {
                "not" | "is" | "where" => Specificity::new(0, 0, 0), // Simplified
                _ => Specificity::new(0, 1, 0),
            };

            Ok(Some((
                SelectorPart::PseudoClass { name, args },
                specificity,
            )))
        }
    }

    fn parse_pseudo_args(&mut self) -> CssResult<String> {
        let mut args = String::new();
        let mut paren_depth = 1;

        loop {
            match self.advance()? {
                Some(Token::LeftParen) => {
                    paren_depth += 1;
                    args.push('(');
                }
                Some(Token::RightParen) => {
                    paren_depth -= 1;
                    if paren_depth == 0 {
                        break;
                    }
                    args.push(')');
                }
                Some(Token::Ident(s)) => args.push_str(&s),
                Some(Token::Number(n)) => args.push_str(&n.to_string()),
                Some(Token::Dimension(n, u)) => args.push_str(&format!("{}{}", n, u)),
                Some(Token::Whitespace) => args.push(' '),
                Some(Token::Delim(c)) => args.push(c),
                Some(Token::Comma) => args.push(','),
                Some(Token::Colon) => args.push(':'),
                Some(Token::String(s)) => {
                    args.push('"');
                    args.push_str(&s);
                    args.push('"');
                }
                Some(Token::Eof) | None => {
                    return Err(CssError::InvalidSelector {
                        selector: format!("({}", args),
                        location: self.location(),
                    });
                }
                Some(token) => {
                    // Handle other tokens
                    args.push_str(&format!("{:?}", token));
                }
            }
        }

        Ok(args.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_selector() {
        let sel = Selector::parse("div").unwrap();
        assert_eq!(sel.parts.len(), 1);
        assert!(matches!(&sel.parts[0], SelectorPart::Type(t) if t == "div"));
        assert_eq!(sel.specificity, Specificity::new(0, 0, 1));
    }

    #[test]
    fn test_class_selector() {
        let sel = Selector::parse(".container").unwrap();
        assert_eq!(sel.parts.len(), 1);
        assert!(matches!(&sel.parts[0], SelectorPart::Class(c) if c == "container"));
        assert_eq!(sel.specificity, Specificity::new(0, 1, 0));
    }

    #[test]
    fn test_id_selector() {
        let sel = Selector::parse("#main").unwrap();
        assert_eq!(sel.parts.len(), 1);
        assert!(matches!(&sel.parts[0], SelectorPart::Id(id) if id == "main"));
        assert_eq!(sel.specificity, Specificity::new(1, 0, 0));
    }

    #[test]
    fn test_universal_selector() {
        let sel = Selector::parse("*").unwrap();
        assert_eq!(sel.parts.len(), 1);
        assert!(matches!(sel.parts[0], SelectorPart::Universal));
        assert_eq!(sel.specificity, Specificity::new(0, 0, 0));
    }

    #[test]
    fn test_compound_selector() {
        let sel = Selector::parse("div.container#main").unwrap();
        assert_eq!(sel.parts.len(), 3);
        assert!(matches!(&sel.parts[0], SelectorPart::Type(t) if t == "div"));
        assert!(matches!(&sel.parts[1], SelectorPart::Class(c) if c == "container"));
        assert!(matches!(&sel.parts[2], SelectorPart::Id(id) if id == "main"));
        assert_eq!(sel.specificity, Specificity::new(1, 1, 1));
    }

    #[test]
    fn test_descendant_combinator() {
        let sel = Selector::parse("div p").unwrap();
        assert_eq!(sel.parts.len(), 3);
        assert!(matches!(&sel.parts[0], SelectorPart::Type(t) if t == "div"));
        assert!(matches!(sel.parts[1], SelectorPart::Combinator(Combinator::Descendant)));
        assert!(matches!(&sel.parts[2], SelectorPart::Type(t) if t == "p"));
    }

    #[test]
    fn test_child_combinator() {
        let sel = Selector::parse("div > p").unwrap();
        assert_eq!(sel.parts.len(), 3);
        assert!(matches!(&sel.parts[0], SelectorPart::Type(t) if t == "div"));
        assert!(matches!(sel.parts[1], SelectorPart::Combinator(Combinator::Child)));
        assert!(matches!(&sel.parts[2], SelectorPart::Type(t) if t == "p"));
    }

    #[test]
    fn test_sibling_combinators() {
        let sel = Selector::parse("h1 + p").unwrap();
        assert!(matches!(sel.parts[1], SelectorPart::Combinator(Combinator::NextSibling)));

        let sel = Selector::parse("h1 ~ p").unwrap();
        assert!(matches!(sel.parts[1], SelectorPart::Combinator(Combinator::SubsequentSibling)));
    }

    #[test]
    fn test_attribute_selector_exists() {
        let sel = Selector::parse("[disabled]").unwrap();
        assert_eq!(sel.parts.len(), 1);
        assert!(matches!(&sel.parts[0], SelectorPart::Attribute { name, op: None, .. } if name == "disabled"));
    }

    #[test]
    fn test_attribute_selector_equals() {
        let sel = Selector::parse("[type=\"text\"]").unwrap();
        assert!(matches!(
            &sel.parts[0],
            SelectorPart::Attribute { name, op: Some(AttributeOp::Equals), value: Some(v), .. }
            if name == "type" && v == "text"
        ));
    }

    #[test]
    fn test_attribute_selector_prefix() {
        let sel = Selector::parse("[href^=\"https\"]").unwrap();
        assert!(matches!(
            &sel.parts[0],
            SelectorPart::Attribute { op: Some(AttributeOp::PrefixMatch), .. }
        ));
    }

    #[test]
    fn test_pseudo_class() {
        let sel = Selector::parse(":hover").unwrap();
        assert!(matches!(&sel.parts[0], SelectorPart::PseudoClass { name, args: None } if name == "hover"));
        assert_eq!(sel.specificity, Specificity::new(0, 1, 0));
    }

    #[test]
    fn test_pseudo_class_functional() {
        let sel = Selector::parse(":nth-child(2n+1)").unwrap();
        assert!(matches!(
            &sel.parts[0],
            SelectorPart::PseudoClass { name, args: Some(_) }
            if name == "nth-child"
        ));
    }

    #[test]
    fn test_pseudo_element() {
        let sel = Selector::parse("::before").unwrap();
        assert!(matches!(&sel.parts[0], SelectorPart::PseudoElement { name, .. } if name == "before"));
        assert_eq!(sel.specificity, Specificity::new(0, 0, 1));
    }

    #[test]
    fn test_pseudo_element_legacy() {
        // Legacy single-colon syntax
        let sel = Selector::parse(":before").unwrap();
        assert!(matches!(&sel.parts[0], SelectorPart::PseudoElement { name, .. } if name == "before"));
    }

    #[test]
    fn test_selector_list() {
        let selectors = Selector::parse_list("div, .class, #id").unwrap();
        assert_eq!(selectors.len(), 3);
        assert!(matches!(&selectors[0].parts[0], SelectorPart::Type(t) if t == "div"));
        assert!(matches!(&selectors[1].parts[0], SelectorPart::Class(c) if c == "class"));
        assert!(matches!(&selectors[2].parts[0], SelectorPart::Id(id) if id == "id"));
    }

    #[test]
    fn test_complex_selector() {
        let sel = Selector::parse("div.container > p.intro:first-child").unwrap();
        // div.container > p.intro:first-child
        // Should have: Type, Class, Combinator, Type, Class, PseudoClass
        assert!(sel.parts.len() >= 5);
    }

    #[test]
    fn test_specificity_comparison() {
        let a = Specificity::new(1, 0, 0);
        let b = Specificity::new(0, 10, 0);
        let c = Specificity::new(0, 0, 100);

        assert!(a > b);
        assert!(b > c);
        assert!(a > c);
    }

    #[test]
    fn test_case_insensitive_attribute() {
        let sel = Selector::parse("[type=\"text\" i]").unwrap();
        assert!(matches!(
            &sel.parts[0],
            SelectorPart::Attribute { case_insensitive: true, .. }
        ));
    }
}

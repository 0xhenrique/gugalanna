//! Gugalanna CSS Parser
//!
//! CSS tokenizer and stylesheet parser.

// TODO: Epic 3 - CSS Parsing
// - CSS tokenizer
// - Selector parser
// - Declaration parser
// - Rule parser

/// Placeholder for CSS stylesheet
#[derive(Debug, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// A CSS rule
#[derive(Debug)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// A CSS selector
#[derive(Debug)]
pub struct Selector {
    pub parts: Vec<SelectorPart>,
}

/// Part of a selector
#[derive(Debug)]
pub enum SelectorPart {
    Universal,
    Type(String),
    Class(String),
    Id(String),
    Attribute { name: String, op: Option<String>, value: Option<String> },
    PseudoClass(String),
    Combinator(Combinator),
}

/// Selector combinator
#[derive(Debug)]
pub enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

/// A CSS declaration (property: value)
#[derive(Debug)]
pub struct Declaration {
    pub property: String,
    pub value: CssValue,
    pub important: bool,
}

/// A CSS value
#[derive(Debug, Clone)]
pub enum CssValue {
    Keyword(String),
    Length(f32, LengthUnit),
    Percentage(f32),
    Color(Color),
    Number(f32),
    String(String),
    Url(String),
}

/// Length units
#[derive(Debug, Clone, Copy)]
pub enum LengthUnit {
    Px,
    Em,
    Rem,
    Percent,
    Vh,
    Vw,
}

/// Color value
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    pub fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }
}

impl Stylesheet {
    pub fn new() -> Self {
        Self::default()
    }
}

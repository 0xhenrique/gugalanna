//! HTML5 Tokenizer
//!
//! Converts HTML text into a stream of tokens.

use smallvec::SmallVec;
use std::collections::VecDeque;

use crate::entities::{decode_entity, decode_numeric};
use crate::error::HtmlResult;

/// An HTML token
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// DOCTYPE declaration
    Doctype {
        name: String,
        public_id: Option<String>,
        system_id: Option<String>,
        force_quirks: bool,
    },
    /// Start tag
    StartTag {
        name: String,
        attributes: SmallVec<[(String, String); 4]>,
        self_closing: bool,
    },
    /// End tag
    EndTag {
        name: String,
    },
    /// Character data
    Character(char),
    /// Comment
    Comment(String),
    /// End of file
    Eof,
}

/// Tokenizer state machine states
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Data,
    TagOpen,
    EndTagOpen,
    TagName,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValueQuoted,
    SelfClosingStartTag,
    BogusComment,
    MarkupDeclarationOpen,
    CommentStart,
    CommentStartDash,
    Comment,
    CommentEndDash,
    CommentEnd,
    Doctype,
    BeforeDoctypeName,
    DoctypeName,
    AfterDoctypeName,
    RawText,
    RawTextLessThan,
    RawTextEndTagOpen,
    RawTextEndTagName,
}

/// HTML5 tokenizer
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    state: State,
    #[allow(dead_code)] // Reserved for future use (character reference states)
    return_state: State,
    tokens: VecDeque<Token>,

    // Current token being built
    current_tag_name: String,
    current_tag_is_end: bool,
    current_tag_self_closing: bool,
    current_attributes: SmallVec<[(String, String); 4]>,
    current_attr_name: String,
    current_attr_value: String,
    current_comment: String,
    current_doctype_name: String,
    current_doctype_public: Option<String>,
    current_doctype_system: Option<String>,
    current_doctype_force_quirks: bool,

    // For raw text (script, style) handling
    last_start_tag: String,
    temp_buffer: String,
}

impl Tokenizer {
    /// Create a new tokenizer for the given input
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
            state: State::Data,
            return_state: State::Data,
            tokens: VecDeque::new(),

            current_tag_name: String::new(),
            current_tag_is_end: false,
            current_tag_self_closing: false,
            current_attributes: SmallVec::new(),
            current_attr_name: String::new(),
            current_attr_value: String::new(),
            current_comment: String::new(),
            current_doctype_name: String::new(),
            current_doctype_public: None,
            current_doctype_system: None,
            current_doctype_force_quirks: false,

            last_start_tag: String::new(),
            temp_buffer: String::new(),
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> HtmlResult<Token> {
        while self.tokens.is_empty() {
            if self.pos >= self.input.len() {
                return Ok(Token::Eof);
            }
            self.step()?;
        }
        Ok(self.tokens.pop_front().unwrap_or(Token::Eof))
    }

    /// Peek at the current character without consuming
    fn current_char(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    /// Consume and return the current character
    fn consume(&mut self) -> Option<char> {
        let c = self.current_char()?;
        self.pos += 1;
        Some(c)
    }

    /// Peek at the next n characters
    fn peek_str(&self, n: usize) -> String {
        self.input[self.pos..].iter().take(n).collect()
    }

    /// Check if we're at a specific string (case-insensitive)
    fn at_str_ci(&self, s: &str) -> bool {
        self.peek_str(s.len()).eq_ignore_ascii_case(s)
    }

    /// Emit a token
    fn emit(&mut self, token: Token) {
        if let Token::StartTag { ref name, .. } = token {
            self.last_start_tag = name.clone();
        }
        self.tokens.push_back(token);
    }

    /// Emit the current tag
    fn emit_current_tag(&mut self) {
        if self.current_tag_is_end {
            self.emit(Token::EndTag {
                name: self.current_tag_name.clone(),
            });
        } else {
            self.emit(Token::StartTag {
                name: self.current_tag_name.clone(),
                attributes: self.current_attributes.clone(),
                self_closing: self.current_tag_self_closing,
            });
        }
        self.reset_tag();
    }

    /// Reset tag state
    fn reset_tag(&mut self) {
        self.current_tag_name.clear();
        self.current_tag_is_end = false;
        self.current_tag_self_closing = false;
        self.current_attributes.clear();
        self.current_attr_name.clear();
        self.current_attr_value.clear();
    }

    /// Push current attribute
    fn push_attribute(&mut self) {
        if !self.current_attr_name.is_empty() {
            self.current_attributes.push((
                self.current_attr_name.to_ascii_lowercase(),
                self.current_attr_value.clone(),
            ));
        }
        self.current_attr_name.clear();
        self.current_attr_value.clear();
    }

    /// Execute one step of the state machine
    fn step(&mut self) -> HtmlResult<()> {
        match self.state {
            State::Data => self.data_state(),
            State::TagOpen => self.tag_open_state(),
            State::EndTagOpen => self.end_tag_open_state(),
            State::TagName => self.tag_name_state(),
            State::BeforeAttributeName => self.before_attribute_name_state(),
            State::AttributeName => self.attribute_name_state(),
            State::AfterAttributeName => self.after_attribute_name_state(),
            State::BeforeAttributeValue => self.before_attribute_value_state(),
            State::AttributeValueDoubleQuoted => self.attribute_value_double_quoted_state(),
            State::AttributeValueSingleQuoted => self.attribute_value_single_quoted_state(),
            State::AttributeValueUnquoted => self.attribute_value_unquoted_state(),
            State::AfterAttributeValueQuoted => self.after_attribute_value_quoted_state(),
            State::SelfClosingStartTag => self.self_closing_start_tag_state(),
            State::BogusComment => self.bogus_comment_state(),
            State::MarkupDeclarationOpen => self.markup_declaration_open_state(),
            State::CommentStart => self.comment_start_state(),
            State::CommentStartDash => self.comment_start_dash_state(),
            State::Comment => self.comment_state(),
            State::CommentEndDash => self.comment_end_dash_state(),
            State::CommentEnd => self.comment_end_state(),
            State::Doctype => self.doctype_state(),
            State::BeforeDoctypeName => self.before_doctype_name_state(),
            State::DoctypeName => self.doctype_name_state(),
            State::AfterDoctypeName => self.after_doctype_name_state(),
            State::RawText => self.raw_text_state(),
            State::RawTextLessThan => self.raw_text_less_than_state(),
            State::RawTextEndTagOpen => self.raw_text_end_tag_open_state(),
            State::RawTextEndTagName => self.raw_text_end_tag_name_state(),
        }
        Ok(())
    }

    // State implementations

    fn data_state(&mut self) {
        match self.consume() {
            Some('<') => self.state = State::TagOpen,
            Some('&') => {
                if let Some(decoded) = self.consume_entity() {
                    for c in decoded.chars() {
                        self.emit(Token::Character(c));
                    }
                } else {
                    self.emit(Token::Character('&'));
                }
            }
            Some(c) => self.emit(Token::Character(c)),
            None => self.emit(Token::Eof),
        }
    }

    fn tag_open_state(&mut self) {
        match self.current_char() {
            Some('!') => {
                self.consume();
                self.state = State::MarkupDeclarationOpen;
            }
            Some('/') => {
                self.consume();
                self.state = State::EndTagOpen;
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.current_tag_is_end = false;
                self.state = State::TagName;
            }
            Some('?') => {
                self.current_comment.clear();
                self.state = State::BogusComment;
            }
            _ => {
                self.emit(Token::Character('<'));
                self.state = State::Data;
            }
        }
    }

    fn end_tag_open_state(&mut self) {
        match self.current_char() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.current_tag_is_end = true;
                self.state = State::TagName;
            }
            Some('>') => {
                self.consume();
                self.state = State::Data;
            }
            None => {
                self.emit(Token::Character('<'));
                self.emit(Token::Character('/'));
                self.emit(Token::Eof);
            }
            _ => {
                self.current_comment.clear();
                self.state = State::BogusComment;
            }
        }
    }

    fn tag_name_state(&mut self) {
        match self.consume() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.state = State::BeforeAttributeName;
            }
            Some('/') => self.state = State::SelfClosingStartTag,
            Some('>') => {
                self.emit_current_tag();
                // Switch to raw text mode for script/style
                if !self.current_tag_is_end
                    && (self.last_start_tag == "script" || self.last_start_tag == "style")
                {
                    self.state = State::RawText;
                } else {
                    self.state = State::Data;
                }
            }
            Some(c) => self.current_tag_name.push(c.to_ascii_lowercase()),
            None => self.emit(Token::Eof),
        }
    }

    fn before_attribute_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
            }
            Some('/') | Some('>') | None => self.state = State::AfterAttributeName,
            Some('=') => {
                self.consume();
                self.current_attr_name.push('=');
                self.state = State::AttributeName;
            }
            _ => {
                self.current_attr_name.clear();
                self.current_attr_value.clear();
                self.state = State::AttributeName;
            }
        }
    }

    fn attribute_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') | Some('/') | Some('>') | None => {
                self.push_attribute();
                self.state = State::AfterAttributeName;
            }
            Some('=') => {
                self.consume();
                self.state = State::BeforeAttributeValue;
            }
            Some(c) => {
                self.consume();
                self.current_attr_name.push(c.to_ascii_lowercase());
            }
        }
    }

    fn after_attribute_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
            }
            Some('/') => {
                self.consume();
                self.state = State::SelfClosingStartTag;
            }
            Some('=') => {
                self.consume();
                self.state = State::BeforeAttributeValue;
            }
            Some('>') => {
                self.consume();
                self.emit_current_tag();
                if !self.current_tag_is_end
                    && (self.last_start_tag == "script" || self.last_start_tag == "style")
                {
                    self.state = State::RawText;
                } else {
                    self.state = State::Data;
                }
            }
            None => self.emit(Token::Eof),
            _ => {
                self.current_attr_name.clear();
                self.current_attr_value.clear();
                self.state = State::AttributeName;
            }
        }
    }

    fn before_attribute_value_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
            }
            Some('"') => {
                self.consume();
                self.state = State::AttributeValueDoubleQuoted;
            }
            Some('\'') => {
                self.consume();
                self.state = State::AttributeValueSingleQuoted;
            }
            Some('>') => {
                self.consume();
                self.push_attribute();
                self.emit_current_tag();
                self.state = State::Data;
            }
            _ => self.state = State::AttributeValueUnquoted,
        }
    }

    fn attribute_value_double_quoted_state(&mut self) {
        match self.consume() {
            Some('"') => {
                self.push_attribute();
                self.state = State::AfterAttributeValueQuoted;
            }
            Some('&') => {
                if let Some(decoded) = self.consume_entity() {
                    self.current_attr_value.push_str(&decoded);
                } else {
                    self.current_attr_value.push('&');
                }
            }
            Some(c) => self.current_attr_value.push(c),
            None => self.emit(Token::Eof),
        }
    }

    fn attribute_value_single_quoted_state(&mut self) {
        match self.consume() {
            Some('\'') => {
                self.push_attribute();
                self.state = State::AfterAttributeValueQuoted;
            }
            Some('&') => {
                if let Some(decoded) = self.consume_entity() {
                    self.current_attr_value.push_str(&decoded);
                } else {
                    self.current_attr_value.push('&');
                }
            }
            Some(c) => self.current_attr_value.push(c),
            None => self.emit(Token::Eof),
        }
    }

    fn attribute_value_unquoted_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
                self.push_attribute();
                self.state = State::BeforeAttributeName;
            }
            Some('&') => {
                self.consume();
                if let Some(decoded) = self.consume_entity() {
                    self.current_attr_value.push_str(&decoded);
                } else {
                    self.current_attr_value.push('&');
                }
            }
            Some('>') => {
                self.consume();
                self.push_attribute();
                self.emit_current_tag();
                self.state = State::Data;
            }
            Some(c) => {
                self.consume();
                self.current_attr_value.push(c);
            }
            None => self.emit(Token::Eof),
        }
    }

    fn after_attribute_value_quoted_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
                self.state = State::BeforeAttributeName;
            }
            Some('/') => {
                self.consume();
                self.state = State::SelfClosingStartTag;
            }
            Some('>') => {
                self.consume();
                self.emit_current_tag();
                if !self.current_tag_is_end
                    && (self.last_start_tag == "script" || self.last_start_tag == "style")
                {
                    self.state = State::RawText;
                } else {
                    self.state = State::Data;
                }
            }
            None => self.emit(Token::Eof),
            _ => self.state = State::BeforeAttributeName,
        }
    }

    fn self_closing_start_tag_state(&mut self) {
        match self.current_char() {
            Some('>') => {
                self.consume();
                self.current_tag_self_closing = true;
                self.emit_current_tag();
                self.state = State::Data;
            }
            None => self.emit(Token::Eof),
            _ => self.state = State::BeforeAttributeName,
        }
    }

    fn bogus_comment_state(&mut self) {
        match self.consume() {
            Some('>') => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.state = State::Data;
            }
            Some(c) => self.current_comment.push(c),
            None => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.emit(Token::Eof);
            }
        }
    }

    fn markup_declaration_open_state(&mut self) {
        if self.at_str_ci("--") {
            self.pos += 2;
            self.current_comment.clear();
            self.state = State::CommentStart;
        } else if self.at_str_ci("DOCTYPE") {
            self.pos += 7;
            self.state = State::Doctype;
        } else if self.at_str_ci("[CDATA[") {
            // We don't fully support CDATA, treat as bogus comment
            self.pos += 7;
            self.current_comment.clear();
            self.state = State::BogusComment;
        } else {
            self.current_comment.clear();
            self.state = State::BogusComment;
        }
    }

    fn comment_start_state(&mut self) {
        match self.current_char() {
            Some('-') => {
                self.consume();
                self.state = State::CommentStartDash;
            }
            Some('>') => {
                self.consume();
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.state = State::Data;
            }
            _ => self.state = State::Comment,
        }
    }

    fn comment_start_dash_state(&mut self) {
        match self.current_char() {
            Some('-') => {
                self.consume();
                self.state = State::CommentEnd;
            }
            Some('>') => {
                self.consume();
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.state = State::Data;
            }
            None => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.emit(Token::Eof);
            }
            _ => {
                self.current_comment.push('-');
                self.state = State::Comment;
            }
        }
    }

    fn comment_state(&mut self) {
        match self.consume() {
            Some('-') => self.state = State::CommentEndDash,
            Some(c) => self.current_comment.push(c),
            None => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.emit(Token::Eof);
            }
        }
    }

    fn comment_end_dash_state(&mut self) {
        match self.current_char() {
            Some('-') => {
                self.consume();
                self.state = State::CommentEnd;
            }
            None => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.emit(Token::Eof);
            }
            _ => {
                self.current_comment.push('-');
                self.state = State::Comment;
            }
        }
    }

    fn comment_end_state(&mut self) {
        match self.current_char() {
            Some('>') => {
                self.consume();
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.state = State::Data;
            }
            Some('-') => {
                self.consume();
                self.current_comment.push('-');
            }
            None => {
                self.emit(Token::Comment(self.current_comment.clone()));
                self.current_comment.clear();
                self.emit(Token::Eof);
            }
            _ => {
                self.current_comment.push_str("--");
                self.state = State::Comment;
            }
        }
    }

    fn doctype_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
                self.state = State::BeforeDoctypeName;
            }
            Some('>') => self.state = State::BeforeDoctypeName,
            None => {
                self.emit(Token::Doctype {
                    name: String::new(),
                    public_id: None,
                    system_id: None,
                    force_quirks: true,
                });
                self.emit(Token::Eof);
            }
            _ => self.state = State::BeforeDoctypeName,
        }
    }

    fn before_doctype_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
            }
            Some('>') => {
                self.consume();
                self.emit(Token::Doctype {
                    name: String::new(),
                    public_id: None,
                    system_id: None,
                    force_quirks: true,
                });
                self.state = State::Data;
            }
            None => {
                self.emit(Token::Doctype {
                    name: String::new(),
                    public_id: None,
                    system_id: None,
                    force_quirks: true,
                });
                self.emit(Token::Eof);
            }
            _ => {
                self.current_doctype_name.clear();
                self.current_doctype_public = None;
                self.current_doctype_system = None;
                self.current_doctype_force_quirks = false;
                self.state = State::DoctypeName;
            }
        }
    }

    fn doctype_name_state(&mut self) {
        match self.consume() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.state = State::AfterDoctypeName;
            }
            Some('>') => {
                self.emit(Token::Doctype {
                    name: self.current_doctype_name.clone(),
                    public_id: self.current_doctype_public.clone(),
                    system_id: self.current_doctype_system.clone(),
                    force_quirks: self.current_doctype_force_quirks,
                });
                self.state = State::Data;
            }
            Some(c) => self.current_doctype_name.push(c.to_ascii_lowercase()),
            None => {
                self.current_doctype_force_quirks = true;
                self.emit(Token::Doctype {
                    name: self.current_doctype_name.clone(),
                    public_id: self.current_doctype_public.clone(),
                    system_id: self.current_doctype_system.clone(),
                    force_quirks: self.current_doctype_force_quirks,
                });
                self.emit(Token::Eof);
            }
        }
    }

    fn after_doctype_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                self.consume();
            }
            Some('>') => {
                self.consume();
                self.emit(Token::Doctype {
                    name: self.current_doctype_name.clone(),
                    public_id: self.current_doctype_public.clone(),
                    system_id: self.current_doctype_system.clone(),
                    force_quirks: self.current_doctype_force_quirks,
                });
                self.state = State::Data;
            }
            None => {
                self.current_doctype_force_quirks = true;
                self.emit(Token::Doctype {
                    name: self.current_doctype_name.clone(),
                    public_id: self.current_doctype_public.clone(),
                    system_id: self.current_doctype_system.clone(),
                    force_quirks: self.current_doctype_force_quirks,
                });
                self.emit(Token::Eof);
            }
            _ => {
                // Skip PUBLIC/SYSTEM for now, just consume until >
                while let Some(c) = self.consume() {
                    if c == '>' {
                        self.emit(Token::Doctype {
                            name: self.current_doctype_name.clone(),
                            public_id: self.current_doctype_public.clone(),
                            system_id: self.current_doctype_system.clone(),
                            force_quirks: self.current_doctype_force_quirks,
                        });
                        self.state = State::Data;
                        return;
                    }
                }
            }
        }
    }

    fn raw_text_state(&mut self) {
        match self.consume() {
            Some('<') => {
                self.state = State::RawTextLessThan;
            }
            Some(c) => self.emit(Token::Character(c)),
            None => self.emit(Token::Eof),
        }
    }

    fn raw_text_less_than_state(&mut self) {
        match self.current_char() {
            Some('/') => {
                self.consume();
                self.temp_buffer.clear();
                self.state = State::RawTextEndTagOpen;
            }
            _ => {
                self.emit(Token::Character('<'));
                self.state = State::RawText;
            }
        }
    }

    fn raw_text_end_tag_open_state(&mut self) {
        match self.current_char() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.current_tag_name.clear();
                self.current_tag_is_end = true;
                self.state = State::RawTextEndTagName;
            }
            _ => {
                self.emit(Token::Character('<'));
                self.emit(Token::Character('/'));
                self.state = State::RawText;
            }
        }
    }

    fn raw_text_end_tag_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.state = State::BeforeAttributeName;
                } else {
                    self.emit_raw_text_chars();
                }
            }
            Some('/') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.state = State::SelfClosingStartTag;
                } else {
                    self.emit_raw_text_chars();
                }
            }
            Some('>') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.emit_current_tag();
                    self.state = State::Data;
                } else {
                    self.emit_raw_text_chars();
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.consume();
                self.current_tag_name.push(c.to_ascii_lowercase());
                self.temp_buffer.push(c);
            }
            _ => {
                self.emit_raw_text_chars();
            }
        }
    }

    fn emit_raw_text_chars(&mut self) {
        self.emit(Token::Character('<'));
        self.emit(Token::Character('/'));
        let temp_chars: Vec<char> = self.temp_buffer.chars().collect();
        let tag_chars: Vec<char> = self.current_tag_name.chars().collect();
        for c in temp_chars {
            self.emit(Token::Character(c));
        }
        for c in tag_chars {
            self.emit(Token::Character(c));
        }
        self.temp_buffer.clear();
        self.current_tag_name.clear();
        self.state = State::RawText;
    }

    /// Try to consume an HTML entity
    fn consume_entity(&mut self) -> Option<String> {
        match self.current_char() {
            Some('#') => {
                self.consume();
                let is_hex = matches!(self.current_char(), Some('x') | Some('X'));
                if is_hex {
                    self.consume();
                }

                let mut num_str = String::new();
                while let Some(c) = self.current_char() {
                    if c == ';' {
                        self.consume();
                        break;
                    }
                    if is_hex && c.is_ascii_hexdigit() {
                        num_str.push(c);
                        self.consume();
                    } else if !is_hex && c.is_ascii_digit() {
                        num_str.push(c);
                        self.consume();
                    } else {
                        break;
                    }
                }

                if is_hex {
                    num_str.insert(0, 'x');
                }
                decode_numeric(&num_str).map(|c| c.to_string())
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let mut name = String::new();
                let start = self.pos;

                while let Some(c) = self.current_char() {
                    if c == ';' {
                        self.consume();
                        break;
                    }
                    if c.is_ascii_alphanumeric() {
                        name.push(c);
                        self.consume();
                    } else {
                        break;
                    }
                }

                if let Some(decoded) = decode_entity(&name) {
                    Some(decoded.to_string())
                } else {
                    // Rewind if no entity found
                    self.pos = start;
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_element() {
        let mut tokenizer = Tokenizer::new("<div>hello</div>");
        let tokens: Vec<Token> = std::iter::from_fn(|| {
            let tok = tokenizer.next_token().ok()?;
            if tok == Token::Eof {
                None
            } else {
                Some(tok)
            }
        })
        .collect();

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "div"));
        assert!(matches!(&tokens[6], Token::EndTag { name } if name == "div"));
    }

    #[test]
    fn test_attributes() {
        let mut tokenizer = Tokenizer::new(r#"<a href="test" class='foo'>"#);
        let tok = tokenizer.next_token().unwrap();

        if let Token::StartTag { name, attributes, .. } = tok {
            assert_eq!(name, "a");
            assert_eq!(attributes.len(), 2);
            assert!(attributes.iter().any(|(k, v)| k == "href" && v == "test"));
            assert!(attributes.iter().any(|(k, v)| k == "class" && v == "foo"));
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_doctype() {
        let mut tokenizer = Tokenizer::new("<!DOCTYPE html>");
        let tok = tokenizer.next_token().unwrap();

        if let Token::Doctype { name, .. } = tok {
            assert_eq!(name, "html");
        } else {
            panic!("Expected Doctype");
        }
    }

    #[test]
    fn test_comment() {
        let mut tokenizer = Tokenizer::new("<!-- this is a comment -->");
        let tok = tokenizer.next_token().unwrap();

        if let Token::Comment(text) = tok {
            assert_eq!(text, " this is a comment ");
        } else {
            panic!("Expected Comment");
        }
    }
}

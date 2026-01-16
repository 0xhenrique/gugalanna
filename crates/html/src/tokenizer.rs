//! HTML5 Tokenizer
//!
//! Converts HTML text into a stream of tokens.

use smallvec::SmallVec;
use std::collections::VecDeque;

use crate::entities::{decode_entity, decode_numeric};
use crate::error::{HtmlResult, SourceLocation};

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
    // RAWTEXT states (script, style, xmp, iframe, noembed, noframes)
    RawText,
    RawTextLessThan,
    RawTextEndTagOpen,
    RawTextEndTagName,
    // RCDATA states (title, textarea) - allows entity references
    Rcdata,
    RcdataLessThan,
    RcdataEndTagOpen,
    RcdataEndTagName,
}

/// Check if an element should use RAWTEXT parsing (no entity decoding)
fn is_rawtext_element(name: &str) -> bool {
    matches!(name, "script" | "style" | "xmp" | "iframe" | "noembed" | "noframes")
}

/// Check if an element should use RCDATA parsing (with entity decoding)
fn is_rcdata_element(name: &str) -> bool {
    matches!(name, "title" | "textarea")
}

/// HTML5 tokenizer
pub struct Tokenizer {
    input: Vec<char>,
    pos: usize,
    state: State,
    #[allow(dead_code)] // Reserved for future use (character reference states)
    return_state: State,
    tokens: VecDeque<Token>,

    // Position tracking
    line: usize,
    column: usize,

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

            line: 1,
            column: 1,

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

        // Track line/column
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        Some(c)
    }

    /// Get the current source location
    pub fn location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.pos)
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
            State::Rcdata => self.rcdata_state(),
            State::RcdataLessThan => self.rcdata_less_than_state(),
            State::RcdataEndTagOpen => self.rcdata_end_tag_open_state(),
            State::RcdataEndTagName => self.rcdata_end_tag_name_state(),
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
                // Capture whether this is an end tag BEFORE emit (which resets the flag)
                let is_end_tag = self.current_tag_is_end;
                self.emit_current_tag();
                self.state = self.next_state_after_start_tag(is_end_tag);
            }
            Some(c) => self.current_tag_name.push(c.to_ascii_lowercase()),
            None => self.emit(Token::Eof),
        }
    }

    /// Determine the next state after emitting a tag
    /// Only switches to special modes for start tags of raw/rcdata elements
    fn next_state_after_start_tag(&self, was_end_tag: bool) -> State {
        if was_end_tag {
            return State::Data;
        }
        if is_rawtext_element(&self.last_start_tag) {
            State::RawText
        } else if is_rcdata_element(&self.last_start_tag) {
            State::Rcdata
        } else {
            State::Data
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
                let is_end_tag = self.current_tag_is_end;
                self.emit_current_tag();
                self.state = self.next_state_after_start_tag(is_end_tag);
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
                let is_end_tag = self.current_tag_is_end;
                self.emit_current_tag();
                self.state = self.next_state_after_start_tag(is_end_tag);
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
        // Only emit temp_buffer which has the original case characters
        let chars: Vec<char> = self.temp_buffer.chars().collect();
        for c in chars {
            self.emit(Token::Character(c));
        }
        self.temp_buffer.clear();
        self.current_tag_name.clear();
        self.state = State::RawText;
    }

    // RCDATA states - similar to RAWTEXT but allows entity references

    fn rcdata_state(&mut self) {
        match self.consume() {
            Some('<') => {
                self.state = State::RcdataLessThan;
            }
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

    fn rcdata_less_than_state(&mut self) {
        match self.current_char() {
            Some('/') => {
                self.consume();
                self.temp_buffer.clear();
                self.state = State::RcdataEndTagOpen;
            }
            _ => {
                self.emit(Token::Character('<'));
                self.state = State::Rcdata;
            }
        }
    }

    fn rcdata_end_tag_open_state(&mut self) {
        match self.current_char() {
            Some(c) if c.is_ascii_alphabetic() => {
                self.current_tag_name.clear();
                self.current_tag_is_end = true;
                self.state = State::RcdataEndTagName;
            }
            _ => {
                self.emit(Token::Character('<'));
                self.emit(Token::Character('/'));
                self.state = State::Rcdata;
            }
        }
    }

    fn rcdata_end_tag_name_state(&mut self) {
        match self.current_char() {
            Some('\t') | Some('\n') | Some('\x0C') | Some(' ') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.state = State::BeforeAttributeName;
                } else {
                    self.emit_rcdata_chars();
                }
            }
            Some('/') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.state = State::SelfClosingStartTag;
                } else {
                    self.emit_rcdata_chars();
                }
            }
            Some('>') => {
                if self.current_tag_name.eq_ignore_ascii_case(&self.last_start_tag) {
                    self.consume();
                    self.emit_current_tag();
                    self.state = State::Data;
                } else {
                    self.emit_rcdata_chars();
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                self.consume();
                self.current_tag_name.push(c.to_ascii_lowercase());
                self.temp_buffer.push(c);
            }
            _ => {
                self.emit_rcdata_chars();
            }
        }
    }

    fn emit_rcdata_chars(&mut self) {
        self.emit(Token::Character('<'));
        self.emit(Token::Character('/'));
        // Only emit temp_buffer which has the original case characters
        let chars: Vec<char> = self.temp_buffer.chars().collect();
        for c in chars {
            self.emit(Token::Character(c));
        }
        self.temp_buffer.clear();
        self.current_tag_name.clear();
        self.state = State::Rcdata;
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

    // Helper to collect all tokens from a tokenizer
    fn collect_tokens(html: &str) -> Vec<Token> {
        let mut tokenizer = Tokenizer::new(html);
        std::iter::from_fn(|| {
            let tok = tokenizer.next_token().ok()?;
            if tok == Token::Eof {
                None
            } else {
                Some(tok)
            }
        })
        .collect()
    }

    // === RAWTEXT element tests ===

    #[test]
    fn test_script_rawtext() {
        let tokens = collect_tokens("<script>var x = '<div>not a tag</div>';</script>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "script"));
        // The content should be character tokens, not parsed as tags
        let content: String = tokens[1..tokens.len() - 1]
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, "var x = '<div>not a tag</div>';");
        assert!(matches!(&tokens[tokens.len() - 1], Token::EndTag { name } if name == "script"));
    }

    #[test]
    fn test_style_rawtext() {
        let tokens = collect_tokens("<style>.foo { color: red; }</style>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "style"));
        let content: String = tokens[1..tokens.len() - 1]
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, ".foo { color: red; }");
        assert!(matches!(&tokens[tokens.len() - 1], Token::EndTag { name } if name == "style"));
    }

    // === RCDATA element tests ===

    #[test]
    fn test_title_rcdata() {
        let tokens = collect_tokens("<title>Hello &amp; World</title>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "title"));
        // RCDATA should decode entities
        let content: String = tokens[1..tokens.len() - 1]
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, "Hello & World"); // Entity decoded
        assert!(matches!(&tokens[tokens.len() - 1], Token::EndTag { name } if name == "title"));
    }

    #[test]
    fn test_textarea_rcdata() {
        let tokens = collect_tokens("<textarea>&lt;script&gt;alert('xss')&lt;/script&gt;</textarea>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "textarea"));
        let content: String = tokens[1..tokens.len() - 1]
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        // Should decode entities but not parse tags
        assert_eq!(content, "<script>alert('xss')</script>");
        assert!(matches!(&tokens[tokens.len() - 1], Token::EndTag { name } if name == "textarea"));
    }

    #[test]
    fn test_title_no_nested_tags() {
        let tokens = collect_tokens("<title><b>Bold</b> title</title>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "title"));
        // Tags inside title should be literal text, not parsed
        let content: String = tokens[1..tokens.len() - 1]
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, "<b>Bold</b> title");
    }

    // === Entity tests ===

    #[test]
    fn test_entity_in_text() {
        let tokens = collect_tokens("<p>&lt;hello&gt;</p>");

        let content: String = tokens
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, "<hello>");
    }

    #[test]
    fn test_entity_in_attribute() {
        let tokens = collect_tokens(r#"<a href="?foo=1&amp;bar=2">"#);

        if let Token::StartTag { attributes, .. } = &tokens[0] {
            let href = attributes.iter().find(|(k, _)| k == "href").map(|(_, v)| v);
            assert_eq!(href, Some(&"?foo=1&bar=2".to_string()));
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_numeric_entity() {
        let tokens = collect_tokens("<p>&#65;&#x42;&#x43;</p>");

        let content: String = tokens
            .iter()
            .filter_map(|t| if let Token::Character(c) = t { Some(*c) } else { None })
            .collect();
        assert_eq!(content, "ABC");
    }

    // === Self-closing tag tests ===

    #[test]
    fn test_self_closing_tag() {
        let tokens = collect_tokens("<br/><hr /><input type='text'/>");

        assert!(matches!(&tokens[0], Token::StartTag { name, self_closing: true, .. } if name == "br"));
        assert!(matches!(&tokens[1], Token::StartTag { name, self_closing: true, .. } if name == "hr"));
        assert!(matches!(&tokens[2], Token::StartTag { name, self_closing: true, .. } if name == "input"));
    }

    #[test]
    fn test_void_elements() {
        // Void elements don't need closing tags
        let tokens = collect_tokens("<img src='test.png'><br><input>");

        assert!(matches!(&tokens[0], Token::StartTag { name, .. } if name == "img"));
        assert!(matches!(&tokens[1], Token::StartTag { name, .. } if name == "br"));
        assert!(matches!(&tokens[2], Token::StartTag { name, .. } if name == "input"));
    }

    // === Attribute edge cases ===

    #[test]
    fn test_unquoted_attribute() {
        let tokens = collect_tokens("<div class=foo data-x=bar>");

        if let Token::StartTag { attributes, .. } = &tokens[0] {
            assert!(attributes.iter().any(|(k, v)| k == "class" && v == "foo"));
            assert!(attributes.iter().any(|(k, v)| k == "data-x" && v == "bar"));
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_empty_attribute() {
        let tokens = collect_tokens("<input disabled readonly>");

        if let Token::StartTag { attributes, .. } = &tokens[0] {
            assert!(attributes.iter().any(|(k, v)| k == "disabled" && v.is_empty()));
            assert!(attributes.iter().any(|(k, v)| k == "readonly" && v.is_empty()));
        } else {
            panic!("Expected StartTag");
        }
    }

    #[test]
    fn test_attribute_case_insensitive() {
        let tokens = collect_tokens("<DIV CLASS='foo' ID='bar'>");

        if let Token::StartTag { name, attributes, .. } = &tokens[0] {
            assert_eq!(name, "div"); // Tag name lowercased
            assert!(attributes.iter().any(|(k, v)| k == "class" && v == "foo")); // Attr name lowercased
            assert!(attributes.iter().any(|(k, v)| k == "id" && v == "bar"));
        } else {
            panic!("Expected StartTag");
        }
    }

    // === Comment edge cases ===

    #[test]
    fn test_empty_comment() {
        let tokens = collect_tokens("<!---->");

        if let Token::Comment(text) = &tokens[0] {
            assert_eq!(text, "");
        } else {
            panic!("Expected Comment");
        }
    }

    #[test]
    fn test_comment_with_dashes() {
        let tokens = collect_tokens("<!-- -- hello -- -->");

        if let Token::Comment(text) = &tokens[0] {
            assert_eq!(text, " -- hello -- ");
        } else {
            panic!("Expected Comment");
        }
    }

    // === Whitespace handling ===

    #[test]
    fn test_whitespace_between_attributes() {
        let tokens = collect_tokens("<div   class='foo'    id='bar'   >");

        if let Token::StartTag { name, attributes, .. } = &tokens[0] {
            assert_eq!(name, "div");
            assert_eq!(attributes.len(), 2);
        } else {
            panic!("Expected StartTag");
        }
    }

    // === Position tracking ===

    #[test]
    fn test_position_tracking() {
        let mut tokenizer = Tokenizer::new("hello\nworld");

        // Consume all of "hello\n"
        for _ in 0..6 {
            tokenizer.next_token().unwrap();
        }

        let loc = tokenizer.location();
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 1); // Start of second line
    }

    // === Malformed HTML recovery ===

    #[test]
    fn test_unclosed_tag() {
        // Unclosed < should emit as character
        let tokens = collect_tokens("< text");

        assert!(matches!(&tokens[0], Token::Character('<')));
    }

    #[test]
    fn test_invalid_tag_start() {
        let tokens = collect_tokens("<1invalid>");

        // Should treat as text since tag names must start with letter
        assert!(matches!(&tokens[0], Token::Character('<')));
    }

    // === Multiple elements ===

    #[test]
    fn test_nested_elements() {
        let tokens = collect_tokens("<div><span>text</span></div>");

        let tag_names: Vec<String> = tokens
            .iter()
            .filter_map(|t| match t {
                Token::StartTag { name, .. } => Some(format!("<{}>", name)),
                Token::EndTag { name } => Some(format!("</{}>", name)),
                _ => None,
            })
            .collect();

        assert_eq!(tag_names, vec!["<div>", "<span>", "</span>", "</div>"]);
    }

    #[test]
    fn test_sibling_elements() {
        let tokens = collect_tokens("<p>a</p><p>b</p><p>c</p>");

        let tag_count = tokens.iter().filter(|t| matches!(t, Token::StartTag { .. } | Token::EndTag { .. })).count();
        assert_eq!(tag_count, 6); // 3 start + 3 end
    }
}

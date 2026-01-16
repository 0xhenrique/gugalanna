//! HTML parsing error types

use std::fmt;
use thiserror::Error;

/// HTML parsing result type
pub type HtmlResult<T> = Result<T, HtmlError>;

/// Source location in the HTML document
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset from start of document
    pub offset: usize,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// HTML parsing errors
#[derive(Debug, Error)]
pub enum HtmlError {
    #[error("Unexpected character '{character}' at {location}")]
    UnexpectedChar {
        character: char,
        location: SourceLocation,
    },

    #[error("Unexpected end of file at {location}")]
    UnexpectedEof {
        location: SourceLocation,
    },

    #[error("Invalid tag name '{name}' at {location}")]
    InvalidTagName {
        name: String,
        location: SourceLocation,
    },

    #[error("Duplicate attribute '{name}' at {location}")]
    DuplicateAttribute {
        name: String,
        location: SourceLocation,
    },

    #[error("Missing attribute value at {location}")]
    MissingAttributeValue {
        location: SourceLocation,
    },

    #[error("Invalid entity reference '&{name};' at {location}")]
    InvalidEntityReference {
        name: String,
        location: SourceLocation,
    },

    #[error("Abrupt closing of empty comment at {location}")]
    AbruptClosingOfEmptyComment {
        location: SourceLocation,
    },

    #[error("Nested comment at {location}")]
    NestedComment {
        location: SourceLocation,
    },

    #[error("End tag with attributes at {location}")]
    EndTagWithAttributes {
        location: SourceLocation,
    },

    #[error("End tag with trailing solidus at {location}")]
    EndTagWithTrailingSolidus {
        location: SourceLocation,
    },

    #[error("Parse error: {message} at {location}")]
    ParseError {
        message: String,
        location: SourceLocation,
    },
}

impl HtmlError {
    /// Get the source location of this error
    pub fn location(&self) -> SourceLocation {
        match self {
            Self::UnexpectedChar { location, .. } => *location,
            Self::UnexpectedEof { location } => *location,
            Self::InvalidTagName { location, .. } => *location,
            Self::DuplicateAttribute { location, .. } => *location,
            Self::MissingAttributeValue { location } => *location,
            Self::InvalidEntityReference { location, .. } => *location,
            Self::AbruptClosingOfEmptyComment { location } => *location,
            Self::NestedComment { location } => *location,
            Self::EndTagWithAttributes { location } => *location,
            Self::EndTagWithTrailingSolidus { location } => *location,
            Self::ParseError { location, .. } => *location,
        }
    }

    /// Create an unexpected character error
    pub fn unexpected_char(c: char, location: SourceLocation) -> Self {
        Self::UnexpectedChar { character: c, location }
    }

    /// Create an unexpected EOF error
    pub fn unexpected_eof(location: SourceLocation) -> Self {
        Self::UnexpectedEof { location }
    }

    /// Create a parse error with a custom message
    pub fn parse_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::ParseError { message: message.into(), location }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_display() {
        let loc = SourceLocation::new(10, 5, 100);
        assert_eq!(format!("{}", loc), "10:5");
    }

    #[test]
    fn test_error_display() {
        let loc = SourceLocation::new(1, 10, 9);
        let err = HtmlError::unexpected_char('<', loc);
        assert_eq!(format!("{}", err), "Unexpected character '<' at 1:10");
    }

    #[test]
    fn test_error_location() {
        let loc = SourceLocation::new(5, 3, 50);
        let err = HtmlError::unexpected_eof(loc);
        assert_eq!(err.location(), loc);
    }
}

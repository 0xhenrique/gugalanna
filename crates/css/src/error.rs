//! CSS parsing error types

use std::fmt;
use thiserror::Error;

/// CSS parsing result type
pub type CssResult<T> = Result<T, CssError>;

/// Source location in CSS
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SourceLocation {
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
    /// Byte offset from start
    pub offset: usize,
}

impl SourceLocation {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self { line, column, offset }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// CSS parsing errors
#[derive(Debug, Error)]
pub enum CssError {
    #[error("Unexpected character '{character}' at {location}")]
    UnexpectedChar {
        character: char,
        location: SourceLocation,
    },

    #[error("Unexpected end of file at {location}")]
    UnexpectedEof {
        location: SourceLocation,
    },

    #[error("Invalid selector '{selector}' at {location}")]
    InvalidSelector {
        selector: String,
        location: SourceLocation,
    },

    #[error("Invalid property name '{name}' at {location}")]
    InvalidProperty {
        name: String,
        location: SourceLocation,
    },

    #[error("Invalid value '{value}' for property '{property}' at {location}")]
    InvalidValue {
        property: String,
        value: String,
        location: SourceLocation,
    },

    #[error("Unterminated string at {location}")]
    UnterminatedString {
        location: SourceLocation,
    },

    #[error("Invalid color '{color}' at {location}")]
    InvalidColor {
        color: String,
        location: SourceLocation,
    },

    #[error("Invalid number '{number}' at {location}")]
    InvalidNumber {
        number: String,
        location: SourceLocation,
    },

    #[error("Parse error: {message} at {location}")]
    ParseError {
        message: String,
        location: SourceLocation,
    },
}

impl CssError {
    /// Get the source location of this error
    pub fn location(&self) -> SourceLocation {
        match self {
            Self::UnexpectedChar { location, .. } => *location,
            Self::UnexpectedEof { location } => *location,
            Self::InvalidSelector { location, .. } => *location,
            Self::InvalidProperty { location, .. } => *location,
            Self::InvalidValue { location, .. } => *location,
            Self::UnterminatedString { location } => *location,
            Self::InvalidColor { location, .. } => *location,
            Self::InvalidNumber { location, .. } => *location,
            Self::ParseError { location, .. } => *location,
        }
    }

    pub fn unexpected_char(c: char, location: SourceLocation) -> Self {
        Self::UnexpectedChar { character: c, location }
    }

    pub fn unexpected_eof(location: SourceLocation) -> Self {
        Self::UnexpectedEof { location }
    }

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
        let err = CssError::unexpected_char('@', loc);
        assert_eq!(format!("{}", err), "Unexpected character '@' at 1:10");
    }
}

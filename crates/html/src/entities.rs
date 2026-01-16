//! HTML entity decoding
//!
//! Handles named entities like &amp;, &lt;, &gt;, etc.

use rustc_hash::FxHashMap;
use std::sync::LazyLock;

/// Map of HTML entity names to their character values
static ENTITIES: LazyLock<FxHashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();
    // Common entities
    m.insert("amp", "&");
    m.insert("lt", "<");
    m.insert("gt", ">");
    m.insert("quot", "\"");
    m.insert("apos", "'");
    m.insert("nbsp", "\u{00A0}");
    m.insert("copy", "\u{00A9}");
    m.insert("reg", "\u{00AE}");
    m.insert("trade", "\u{2122}");
    m.insert("mdash", "\u{2014}");
    m.insert("ndash", "\u{2013}");
    m.insert("lsquo", "\u{2018}");
    m.insert("rsquo", "\u{2019}");
    m.insert("ldquo", "\u{201C}");
    m.insert("rdquo", "\u{201D}");
    m.insert("bull", "\u{2022}");
    m.insert("hellip", "\u{2026}");
    m.insert("euro", "\u{20AC}");
    m.insert("pound", "\u{00A3}");
    m.insert("yen", "\u{00A5}");
    m.insert("cent", "\u{00A2}");
    // More can be added as needed
    m
});

/// Decode an HTML entity (without the & and ;)
pub fn decode_entity(name: &str) -> Option<&'static str> {
    ENTITIES.get(name).copied()
}

/// Decode a numeric entity (&#123; or &#x7B;)
pub fn decode_numeric(s: &str) -> Option<char> {
    let value = if let Some(hex) = s.strip_prefix('x').or_else(|| s.strip_prefix('X')) {
        u32::from_str_radix(hex, 16).ok()?
    } else {
        s.parse::<u32>().ok()?
    };
    char::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_entity() {
        assert_eq!(decode_entity("amp"), Some("&"));
        assert_eq!(decode_entity("lt"), Some("<"));
        assert_eq!(decode_entity("unknown"), None);
    }

    #[test]
    fn test_decode_numeric() {
        assert_eq!(decode_numeric("65"), Some('A'));
        assert_eq!(decode_numeric("x41"), Some('A'));
        assert_eq!(decode_numeric("X41"), Some('A'));
    }
}

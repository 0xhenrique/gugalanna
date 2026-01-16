//! HTML entity decoding
//!
//! Handles named entities like &amp;, &lt;, &gt;, etc.

use rustc_hash::FxHashMap;
use std::sync::LazyLock;

/// Map of HTML entity names to their character values
static ENTITIES: LazyLock<FxHashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = FxHashMap::default();

    // === Essential entities ===
    m.insert("amp", "&");
    m.insert("lt", "<");
    m.insert("gt", ">");
    m.insert("quot", "\"");
    m.insert("apos", "'");

    // === ISO 8859-1 (Latin-1) Symbols ===
    m.insert("nbsp", "\u{00A0}");     // non-breaking space
    m.insert("iexcl", "\u{00A1}");    // inverted exclamation mark
    m.insert("cent", "\u{00A2}");     // cent sign
    m.insert("pound", "\u{00A3}");    // pound sign
    m.insert("curren", "\u{00A4}");   // currency sign
    m.insert("yen", "\u{00A5}");      // yen sign
    m.insert("brvbar", "\u{00A6}");   // broken vertical bar
    m.insert("sect", "\u{00A7}");     // section sign
    m.insert("uml", "\u{00A8}");      // diaeresis
    m.insert("copy", "\u{00A9}");     // copyright sign
    m.insert("ordf", "\u{00AA}");     // feminine ordinal indicator
    m.insert("laquo", "\u{00AB}");    // left-pointing double angle quotation mark
    m.insert("not", "\u{00AC}");      // not sign
    m.insert("shy", "\u{00AD}");      // soft hyphen
    m.insert("reg", "\u{00AE}");      // registered sign
    m.insert("macr", "\u{00AF}");     // macron
    m.insert("deg", "\u{00B0}");      // degree sign
    m.insert("plusmn", "\u{00B1}");   // plus-minus sign
    m.insert("sup2", "\u{00B2}");     // superscript two
    m.insert("sup3", "\u{00B3}");     // superscript three
    m.insert("acute", "\u{00B4}");    // acute accent
    m.insert("micro", "\u{00B5}");    // micro sign
    m.insert("para", "\u{00B6}");     // pilcrow sign
    m.insert("middot", "\u{00B7}");   // middle dot
    m.insert("cedil", "\u{00B8}");    // cedilla
    m.insert("sup1", "\u{00B9}");     // superscript one
    m.insert("ordm", "\u{00BA}");     // masculine ordinal indicator
    m.insert("raquo", "\u{00BB}");    // right-pointing double angle quotation mark
    m.insert("frac14", "\u{00BC}");   // vulgar fraction one quarter
    m.insert("frac12", "\u{00BD}");   // vulgar fraction one half
    m.insert("frac34", "\u{00BE}");   // vulgar fraction three quarters
    m.insert("iquest", "\u{00BF}");   // inverted question mark
    m.insert("times", "\u{00D7}");    // multiplication sign
    m.insert("divide", "\u{00F7}");   // division sign

    // === Latin Extended Characters ===
    m.insert("Agrave", "\u{00C0}");
    m.insert("Aacute", "\u{00C1}");
    m.insert("Acirc", "\u{00C2}");
    m.insert("Atilde", "\u{00C3}");
    m.insert("Auml", "\u{00C4}");
    m.insert("Aring", "\u{00C5}");
    m.insert("AElig", "\u{00C6}");
    m.insert("Ccedil", "\u{00C7}");
    m.insert("Egrave", "\u{00C8}");
    m.insert("Eacute", "\u{00C9}");
    m.insert("Ecirc", "\u{00CA}");
    m.insert("Euml", "\u{00CB}");
    m.insert("Igrave", "\u{00CC}");
    m.insert("Iacute", "\u{00CD}");
    m.insert("Icirc", "\u{00CE}");
    m.insert("Iuml", "\u{00CF}");
    m.insert("ETH", "\u{00D0}");
    m.insert("Ntilde", "\u{00D1}");
    m.insert("Ograve", "\u{00D2}");
    m.insert("Oacute", "\u{00D3}");
    m.insert("Ocirc", "\u{00D4}");
    m.insert("Otilde", "\u{00D5}");
    m.insert("Ouml", "\u{00D6}");
    m.insert("Oslash", "\u{00D8}");
    m.insert("Ugrave", "\u{00D9}");
    m.insert("Uacute", "\u{00DA}");
    m.insert("Ucirc", "\u{00DB}");
    m.insert("Uuml", "\u{00DC}");
    m.insert("Yacute", "\u{00DD}");
    m.insert("THORN", "\u{00DE}");
    m.insert("szlig", "\u{00DF}");
    m.insert("agrave", "\u{00E0}");
    m.insert("aacute", "\u{00E1}");
    m.insert("acirc", "\u{00E2}");
    m.insert("atilde", "\u{00E3}");
    m.insert("auml", "\u{00E4}");
    m.insert("aring", "\u{00E5}");
    m.insert("aelig", "\u{00E6}");
    m.insert("ccedil", "\u{00E7}");
    m.insert("egrave", "\u{00E8}");
    m.insert("eacute", "\u{00E9}");
    m.insert("ecirc", "\u{00EA}");
    m.insert("euml", "\u{00EB}");
    m.insert("igrave", "\u{00EC}");
    m.insert("iacute", "\u{00ED}");
    m.insert("icirc", "\u{00EE}");
    m.insert("iuml", "\u{00EF}");
    m.insert("eth", "\u{00F0}");
    m.insert("ntilde", "\u{00F1}");
    m.insert("ograve", "\u{00F2}");
    m.insert("oacute", "\u{00F3}");
    m.insert("ocirc", "\u{00F4}");
    m.insert("otilde", "\u{00F5}");
    m.insert("ouml", "\u{00F6}");
    m.insert("oslash", "\u{00F8}");
    m.insert("ugrave", "\u{00F9}");
    m.insert("uacute", "\u{00FA}");
    m.insert("ucirc", "\u{00FB}");
    m.insert("uuml", "\u{00FC}");
    m.insert("yacute", "\u{00FD}");
    m.insert("thorn", "\u{00FE}");
    m.insert("yuml", "\u{00FF}");
    m.insert("OElig", "\u{0152}");
    m.insert("oelig", "\u{0153}");
    m.insert("Scaron", "\u{0160}");
    m.insert("scaron", "\u{0161}");
    m.insert("Yuml", "\u{0178}");
    m.insert("fnof", "\u{0192}");

    // === Greek Letters ===
    m.insert("Alpha", "\u{0391}");
    m.insert("Beta", "\u{0392}");
    m.insert("Gamma", "\u{0393}");
    m.insert("Delta", "\u{0394}");
    m.insert("Epsilon", "\u{0395}");
    m.insert("Zeta", "\u{0396}");
    m.insert("Eta", "\u{0397}");
    m.insert("Theta", "\u{0398}");
    m.insert("Iota", "\u{0399}");
    m.insert("Kappa", "\u{039A}");
    m.insert("Lambda", "\u{039B}");
    m.insert("Mu", "\u{039C}");
    m.insert("Nu", "\u{039D}");
    m.insert("Xi", "\u{039E}");
    m.insert("Omicron", "\u{039F}");
    m.insert("Pi", "\u{03A0}");
    m.insert("Rho", "\u{03A1}");
    m.insert("Sigma", "\u{03A3}");
    m.insert("Tau", "\u{03A4}");
    m.insert("Upsilon", "\u{03A5}");
    m.insert("Phi", "\u{03A6}");
    m.insert("Chi", "\u{03A7}");
    m.insert("Psi", "\u{03A8}");
    m.insert("Omega", "\u{03A9}");
    m.insert("alpha", "\u{03B1}");
    m.insert("beta", "\u{03B2}");
    m.insert("gamma", "\u{03B3}");
    m.insert("delta", "\u{03B4}");
    m.insert("epsilon", "\u{03B5}");
    m.insert("zeta", "\u{03B6}");
    m.insert("eta", "\u{03B7}");
    m.insert("theta", "\u{03B8}");
    m.insert("iota", "\u{03B9}");
    m.insert("kappa", "\u{03BA}");
    m.insert("lambda", "\u{03BB}");
    m.insert("mu", "\u{03BC}");
    m.insert("nu", "\u{03BD}");
    m.insert("xi", "\u{03BE}");
    m.insert("omicron", "\u{03BF}");
    m.insert("pi", "\u{03C0}");
    m.insert("rho", "\u{03C1}");
    m.insert("sigmaf", "\u{03C2}");
    m.insert("sigma", "\u{03C3}");
    m.insert("tau", "\u{03C4}");
    m.insert("upsilon", "\u{03C5}");
    m.insert("phi", "\u{03C6}");
    m.insert("chi", "\u{03C7}");
    m.insert("psi", "\u{03C8}");
    m.insert("omega", "\u{03C9}");
    m.insert("thetasym", "\u{03D1}");
    m.insert("upsih", "\u{03D2}");
    m.insert("piv", "\u{03D6}");

    // === General Punctuation ===
    m.insert("ensp", "\u{2002}");      // en space
    m.insert("emsp", "\u{2003}");      // em space
    m.insert("thinsp", "\u{2009}");    // thin space
    m.insert("zwnj", "\u{200C}");      // zero width non-joiner
    m.insert("zwj", "\u{200D}");       // zero width joiner
    m.insert("lrm", "\u{200E}");       // left-to-right mark
    m.insert("rlm", "\u{200F}");       // right-to-left mark
    m.insert("ndash", "\u{2013}");     // en dash
    m.insert("mdash", "\u{2014}");     // em dash
    m.insert("lsquo", "\u{2018}");     // left single quotation mark
    m.insert("rsquo", "\u{2019}");     // right single quotation mark
    m.insert("sbquo", "\u{201A}");     // single low-9 quotation mark
    m.insert("ldquo", "\u{201C}");     // left double quotation mark
    m.insert("rdquo", "\u{201D}");     // right double quotation mark
    m.insert("bdquo", "\u{201E}");     // double low-9 quotation mark
    m.insert("dagger", "\u{2020}");    // dagger
    m.insert("Dagger", "\u{2021}");    // double dagger
    m.insert("bull", "\u{2022}");      // bullet
    m.insert("hellip", "\u{2026}");    // horizontal ellipsis
    m.insert("permil", "\u{2030}");    // per mille sign
    m.insert("prime", "\u{2032}");     // prime
    m.insert("Prime", "\u{2033}");     // double prime
    m.insert("lsaquo", "\u{2039}");    // single left-pointing angle quotation mark
    m.insert("rsaquo", "\u{203A}");    // single right-pointing angle quotation mark
    m.insert("oline", "\u{203E}");     // overline
    m.insert("frasl", "\u{2044}");     // fraction slash

    // === Currency Symbols ===
    m.insert("euro", "\u{20AC}");      // euro sign

    // === Letterlike Symbols ===
    m.insert("weierp", "\u{2118}");    // script capital P
    m.insert("image", "\u{2111}");     // black-letter capital I
    m.insert("real", "\u{211C}");      // black-letter capital R
    m.insert("trade", "\u{2122}");     // trade mark sign
    m.insert("alefsym", "\u{2135}");   // alef symbol

    // === Arrows ===
    m.insert("larr", "\u{2190}");      // leftwards arrow
    m.insert("uarr", "\u{2191}");      // upwards arrow
    m.insert("rarr", "\u{2192}");      // rightwards arrow
    m.insert("darr", "\u{2193}");      // downwards arrow
    m.insert("harr", "\u{2194}");      // left right arrow
    m.insert("crarr", "\u{21B5}");     // downwards arrow with corner leftwards
    m.insert("lArr", "\u{21D0}");      // leftwards double arrow
    m.insert("uArr", "\u{21D1}");      // upwards double arrow
    m.insert("rArr", "\u{21D2}");      // rightwards double arrow
    m.insert("dArr", "\u{21D3}");      // downwards double arrow
    m.insert("hArr", "\u{21D4}");      // left right double arrow

    // === Mathematical Operators ===
    m.insert("forall", "\u{2200}");    // for all
    m.insert("part", "\u{2202}");      // partial differential
    m.insert("exist", "\u{2203}");     // there exists
    m.insert("empty", "\u{2205}");     // empty set
    m.insert("nabla", "\u{2207}");     // nabla
    m.insert("isin", "\u{2208}");      // element of
    m.insert("notin", "\u{2209}");     // not an element of
    m.insert("ni", "\u{220B}");        // contains as member
    m.insert("prod", "\u{220F}");      // n-ary product
    m.insert("sum", "\u{2211}");       // n-ary summation
    m.insert("minus", "\u{2212}");     // minus sign
    m.insert("lowast", "\u{2217}");    // asterisk operator
    m.insert("radic", "\u{221A}");     // square root
    m.insert("prop", "\u{221D}");      // proportional to
    m.insert("infin", "\u{221E}");     // infinity
    m.insert("ang", "\u{2220}");       // angle
    m.insert("and", "\u{2227}");       // logical and
    m.insert("or", "\u{2228}");        // logical or
    m.insert("cap", "\u{2229}");       // intersection
    m.insert("cup", "\u{222A}");       // union
    m.insert("int", "\u{222B}");       // integral
    m.insert("there4", "\u{2234}");    // therefore
    m.insert("sim", "\u{223C}");       // tilde operator
    m.insert("cong", "\u{2245}");      // approximately equal to
    m.insert("asymp", "\u{2248}");     // almost equal to
    m.insert("ne", "\u{2260}");        // not equal to
    m.insert("equiv", "\u{2261}");     // identical to
    m.insert("le", "\u{2264}");        // less-than or equal to
    m.insert("ge", "\u{2265}");        // greater-than or equal to
    m.insert("sub", "\u{2282}");       // subset of
    m.insert("sup", "\u{2283}");       // superset of
    m.insert("nsub", "\u{2284}");      // not a subset of
    m.insert("sube", "\u{2286}");      // subset of or equal to
    m.insert("supe", "\u{2287}");      // superset of or equal to
    m.insert("oplus", "\u{2295}");     // circled plus
    m.insert("otimes", "\u{2297}");    // circled times
    m.insert("perp", "\u{22A5}");      // up tack
    m.insert("sdot", "\u{22C5}");      // dot operator

    // === Miscellaneous Technical ===
    m.insert("lceil", "\u{2308}");     // left ceiling
    m.insert("rceil", "\u{2309}");     // right ceiling
    m.insert("lfloor", "\u{230A}");    // left floor
    m.insert("rfloor", "\u{230B}");    // right floor
    m.insert("lang", "\u{2329}");      // left-pointing angle bracket
    m.insert("rang", "\u{232A}");      // right-pointing angle bracket

    // === Geometric Shapes ===
    m.insert("loz", "\u{25CA}");       // lozenge

    // === Miscellaneous Symbols ===
    m.insert("spades", "\u{2660}");    // black spade suit
    m.insert("clubs", "\u{2663}");     // black club suit
    m.insert("hearts", "\u{2665}");    // black heart suit
    m.insert("diams", "\u{2666}");     // black diamond suit

    // === Additional useful entities ===
    m.insert("circ", "\u{02C6}");      // modifier letter circumflex accent
    m.insert("tilde", "\u{02DC}");     // small tilde

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

    // Handle special replacements per HTML5 spec
    let value = match value {
        0x00 => 0xFFFD,                           // NULL -> REPLACEMENT CHARACTER
        0x80 => 0x20AC,                           // EURO SIGN
        0x82 => 0x201A,                           // SINGLE LOW-9 QUOTATION MARK
        0x83 => 0x0192,                           // LATIN SMALL LETTER F WITH HOOK
        0x84 => 0x201E,                           // DOUBLE LOW-9 QUOTATION MARK
        0x85 => 0x2026,                           // HORIZONTAL ELLIPSIS
        0x86 => 0x2020,                           // DAGGER
        0x87 => 0x2021,                           // DOUBLE DAGGER
        0x88 => 0x02C6,                           // MODIFIER LETTER CIRCUMFLEX ACCENT
        0x89 => 0x2030,                           // PER MILLE SIGN
        0x8A => 0x0160,                           // LATIN CAPITAL LETTER S WITH CARON
        0x8B => 0x2039,                           // SINGLE LEFT-POINTING ANGLE QUOTATION MARK
        0x8C => 0x0152,                           // LATIN CAPITAL LIGATURE OE
        0x8E => 0x017D,                           // LATIN CAPITAL LETTER Z WITH CARON
        0x91 => 0x2018,                           // LEFT SINGLE QUOTATION MARK
        0x92 => 0x2019,                           // RIGHT SINGLE QUOTATION MARK
        0x93 => 0x201C,                           // LEFT DOUBLE QUOTATION MARK
        0x94 => 0x201D,                           // RIGHT DOUBLE QUOTATION MARK
        0x95 => 0x2022,                           // BULLET
        0x96 => 0x2013,                           // EN DASH
        0x97 => 0x2014,                           // EM DASH
        0x98 => 0x02DC,                           // SMALL TILDE
        0x99 => 0x2122,                           // TRADE MARK SIGN
        0x9A => 0x0161,                           // LATIN SMALL LETTER S WITH CARON
        0x9B => 0x203A,                           // SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
        0x9C => 0x0153,                           // LATIN SMALL LIGATURE OE
        0x9E => 0x017E,                           // LATIN SMALL LETTER Z WITH CARON
        0x9F => 0x0178,                           // LATIN CAPITAL LETTER Y WITH DIAERESIS
        // Surrogate range is invalid
        0xD800..=0xDFFF => 0xFFFD,
        // Values above max Unicode are invalid
        v if v > 0x10FFFF => 0xFFFD,
        v => v,
    };

    char::from_u32(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_essential_entities() {
        assert_eq!(decode_entity("amp"), Some("&"));
        assert_eq!(decode_entity("lt"), Some("<"));
        assert_eq!(decode_entity("gt"), Some(">"));
        assert_eq!(decode_entity("quot"), Some("\""));
        assert_eq!(decode_entity("apos"), Some("'"));
    }

    #[test]
    fn test_common_entities() {
        assert_eq!(decode_entity("nbsp"), Some("\u{00A0}"));
        assert_eq!(decode_entity("copy"), Some("\u{00A9}"));
        assert_eq!(decode_entity("reg"), Some("\u{00AE}"));
        assert_eq!(decode_entity("trade"), Some("\u{2122}"));
        assert_eq!(decode_entity("euro"), Some("\u{20AC}"));
    }

    #[test]
    fn test_punctuation_entities() {
        assert_eq!(decode_entity("mdash"), Some("\u{2014}"));
        assert_eq!(decode_entity("ndash"), Some("\u{2013}"));
        assert_eq!(decode_entity("lsquo"), Some("\u{2018}"));
        assert_eq!(decode_entity("rsquo"), Some("\u{2019}"));
        assert_eq!(decode_entity("ldquo"), Some("\u{201C}"));
        assert_eq!(decode_entity("rdquo"), Some("\u{201D}"));
        assert_eq!(decode_entity("hellip"), Some("\u{2026}"));
        assert_eq!(decode_entity("bull"), Some("\u{2022}"));
    }

    #[test]
    fn test_math_entities() {
        assert_eq!(decode_entity("plusmn"), Some("\u{00B1}"));
        assert_eq!(decode_entity("times"), Some("\u{00D7}"));
        assert_eq!(decode_entity("divide"), Some("\u{00F7}"));
        assert_eq!(decode_entity("ne"), Some("\u{2260}"));
        assert_eq!(decode_entity("le"), Some("\u{2264}"));
        assert_eq!(decode_entity("ge"), Some("\u{2265}"));
        assert_eq!(decode_entity("infin"), Some("\u{221E}"));
    }

    #[test]
    fn test_greek_letters() {
        assert_eq!(decode_entity("alpha"), Some("\u{03B1}"));
        assert_eq!(decode_entity("beta"), Some("\u{03B2}"));
        assert_eq!(decode_entity("pi"), Some("\u{03C0}"));
        assert_eq!(decode_entity("Omega"), Some("\u{03A9}"));
    }

    #[test]
    fn test_arrow_entities() {
        assert_eq!(decode_entity("larr"), Some("\u{2190}"));
        assert_eq!(decode_entity("rarr"), Some("\u{2192}"));
        assert_eq!(decode_entity("uarr"), Some("\u{2191}"));
        assert_eq!(decode_entity("darr"), Some("\u{2193}"));
    }

    #[test]
    fn test_unknown_entity() {
        assert_eq!(decode_entity("unknown"), None);
        assert_eq!(decode_entity("notanentity"), None);
    }

    #[test]
    fn test_decode_numeric_decimal() {
        assert_eq!(decode_numeric("65"), Some('A'));
        assert_eq!(decode_numeric("97"), Some('a'));
        assert_eq!(decode_numeric("8364"), Some('€'));
        assert_eq!(decode_numeric("169"), Some('©'));
    }

    #[test]
    fn test_decode_numeric_hex() {
        assert_eq!(decode_numeric("x41"), Some('A'));
        assert_eq!(decode_numeric("X41"), Some('A'));
        assert_eq!(decode_numeric("x61"), Some('a'));
        assert_eq!(decode_numeric("x20AC"), Some('€'));
    }

    #[test]
    fn test_decode_numeric_special_replacements() {
        // NULL -> REPLACEMENT CHARACTER
        assert_eq!(decode_numeric("0"), Some('\u{FFFD}'));
        // Windows-1252 to Unicode mappings
        assert_eq!(decode_numeric("128"), Some('€'));      // 0x80 -> euro
        assert_eq!(decode_numeric("145"), Some('\u{2018}')); // 0x91 -> left single quote
        assert_eq!(decode_numeric("146"), Some('\u{2019}')); // 0x92 -> right single quote
    }

    #[test]
    fn test_decode_numeric_invalid() {
        // Surrogate range
        assert_eq!(decode_numeric("55296"), Some('\u{FFFD}')); // 0xD800
        assert_eq!(decode_numeric("57343"), Some('\u{FFFD}')); // 0xDFFF
        // Above max Unicode
        assert_eq!(decode_numeric("1114112"), Some('\u{FFFD}')); // 0x110000
    }
}

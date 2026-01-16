//! CSS Value Parser
//!
//! Parses CSS values including colors, lengths, and other types.

use crate::error::{CssError, CssResult, SourceLocation};
use crate::tokenizer::Token;

/// A CSS value
#[derive(Debug, Clone, PartialEq)]
pub enum CssValue {
    /// Keyword value (e.g., auto, inherit, none)
    Keyword(String),
    /// Length value with unit
    Length(f32, LengthUnit),
    /// Percentage value
    Percentage(f32),
    /// Color value
    Color(Color),
    /// Numeric value (unitless)
    Number(f32),
    /// String value
    String(String),
    /// URL value
    Url(String),
    /// Function call (e.g., calc(), var())
    Function(String, Vec<CssValue>),
    /// Multiple values (space-separated)
    List(Vec<CssValue>),
    /// Comma-separated values
    CommaSeparated(Vec<CssValue>),
}

/// Length units
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthUnit {
    /// Pixels
    Px,
    /// Em units (relative to font-size)
    Em,
    /// Rem units (relative to root font-size)
    Rem,
    /// Viewport width percentage
    Vw,
    /// Viewport height percentage
    Vh,
    /// Viewport minimum
    Vmin,
    /// Viewport maximum
    Vmax,
    /// Centimeters
    Cm,
    /// Millimeters
    Mm,
    /// Inches
    In,
    /// Points (1/72 inch)
    Pt,
    /// Picas (12 points)
    Pc,
    /// Character width (width of '0')
    Ch,
    /// x-height (height of 'x')
    Ex,
}

impl LengthUnit {
    /// Parse a unit string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "px" => Some(LengthUnit::Px),
            "em" => Some(LengthUnit::Em),
            "rem" => Some(LengthUnit::Rem),
            "vw" => Some(LengthUnit::Vw),
            "vh" => Some(LengthUnit::Vh),
            "vmin" => Some(LengthUnit::Vmin),
            "vmax" => Some(LengthUnit::Vmax),
            "cm" => Some(LengthUnit::Cm),
            "mm" => Some(LengthUnit::Mm),
            "in" => Some(LengthUnit::In),
            "pt" => Some(LengthUnit::Pt),
            "pc" => Some(LengthUnit::Pc),
            "ch" => Some(LengthUnit::Ch),
            "ex" => Some(LengthUnit::Ex),
            _ => None,
        }
    }

    /// Convert to pixels (approximate, using common defaults)
    pub fn to_px(&self, value: f32, font_size: f32, root_font_size: f32, viewport_width: f32, viewport_height: f32) -> f32 {
        match self {
            LengthUnit::Px => value,
            LengthUnit::Em => value * font_size,
            LengthUnit::Rem => value * root_font_size,
            LengthUnit::Vw => value * viewport_width / 100.0,
            LengthUnit::Vh => value * viewport_height / 100.0,
            LengthUnit::Vmin => value * viewport_width.min(viewport_height) / 100.0,
            LengthUnit::Vmax => value * viewport_width.max(viewport_height) / 100.0,
            LengthUnit::Cm => value * 37.795, // 1cm ≈ 37.795px at 96dpi
            LengthUnit::Mm => value * 3.7795,
            LengthUnit::In => value * 96.0, // 96px per inch
            LengthUnit::Pt => value * 96.0 / 72.0,
            LengthUnit::Pc => value * 96.0 / 6.0,
            LengthUnit::Ch => value * font_size * 0.5, // Approximate
            LengthUnit::Ex => value * font_size * 0.5, // Approximate
        }
    }
}

/// Color value (RGBA)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn from_rgba_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: (r.clamp(0.0, 255.0)) as u8,
            g: (g.clamp(0.0, 255.0)) as u8,
            b: (b.clamp(0.0, 255.0)) as u8,
            a: (a.clamp(0.0, 1.0) * 255.0) as u8,
        }
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

    /// Parse a hex color string (without #)
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim();
        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                Some(Color::rgb(r * 17, g * 17, b * 17))
            }
            4 => {
                // #RGBA -> #RRGGBBAA
                let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4], 16).ok()?;
                Some(Color::rgba(r * 17, g * 17, b * 17, a * 17))
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Color::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Color::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Get a named color
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            // Basic colors
            "black" => Some(Color::rgb(0, 0, 0)),
            "white" => Some(Color::rgb(255, 255, 255)),
            "red" => Some(Color::rgb(255, 0, 0)),
            "green" => Some(Color::rgb(0, 128, 0)),
            "blue" => Some(Color::rgb(0, 0, 255)),
            "yellow" => Some(Color::rgb(255, 255, 0)),
            "cyan" | "aqua" => Some(Color::rgb(0, 255, 255)),
            "magenta" | "fuchsia" => Some(Color::rgb(255, 0, 255)),

            // Grays
            "gray" | "grey" => Some(Color::rgb(128, 128, 128)),
            "silver" => Some(Color::rgb(192, 192, 192)),
            "darkgray" | "darkgrey" => Some(Color::rgb(169, 169, 169)),
            "lightgray" | "lightgrey" => Some(Color::rgb(211, 211, 211)),
            "dimgray" | "dimgrey" => Some(Color::rgb(105, 105, 105)),

            // Reds
            "maroon" => Some(Color::rgb(128, 0, 0)),
            "darkred" => Some(Color::rgb(139, 0, 0)),
            "crimson" => Some(Color::rgb(220, 20, 60)),
            "indianred" => Some(Color::rgb(205, 92, 92)),
            "lightcoral" => Some(Color::rgb(240, 128, 128)),
            "salmon" => Some(Color::rgb(250, 128, 114)),
            "darksalmon" => Some(Color::rgb(233, 150, 122)),
            "lightsalmon" => Some(Color::rgb(255, 160, 122)),
            "tomato" => Some(Color::rgb(255, 99, 71)),
            "orangered" => Some(Color::rgb(255, 69, 0)),
            "coral" => Some(Color::rgb(255, 127, 80)),

            // Oranges
            "orange" => Some(Color::rgb(255, 165, 0)),
            "darkorange" => Some(Color::rgb(255, 140, 0)),

            // Yellows
            "gold" => Some(Color::rgb(255, 215, 0)),
            "lightyellow" => Some(Color::rgb(255, 255, 224)),
            "lemonchiffon" => Some(Color::rgb(255, 250, 205)),
            "khaki" => Some(Color::rgb(240, 230, 140)),
            "darkkhaki" => Some(Color::rgb(189, 183, 107)),

            // Greens
            "lime" => Some(Color::rgb(0, 255, 0)),
            "limegreen" => Some(Color::rgb(50, 205, 50)),
            "lightgreen" => Some(Color::rgb(144, 238, 144)),
            "palegreen" => Some(Color::rgb(152, 251, 152)),
            "darkgreen" => Some(Color::rgb(0, 100, 0)),
            "forestgreen" => Some(Color::rgb(34, 139, 34)),
            "seagreen" => Some(Color::rgb(46, 139, 87)),
            "olive" => Some(Color::rgb(128, 128, 0)),
            "olivedrab" => Some(Color::rgb(107, 142, 35)),
            "mediumseagreen" => Some(Color::rgb(60, 179, 113)),
            "springgreen" => Some(Color::rgb(0, 255, 127)),
            "mediumspringgreen" => Some(Color::rgb(0, 250, 154)),
            "darkseagreen" => Some(Color::rgb(143, 188, 143)),
            "mediumaquamarine" => Some(Color::rgb(102, 205, 170)),
            "yellowgreen" => Some(Color::rgb(154, 205, 50)),
            "lawngreen" => Some(Color::rgb(124, 252, 0)),
            "chartreuse" => Some(Color::rgb(127, 255, 0)),
            "greenyellow" => Some(Color::rgb(173, 255, 47)),

            // Blues
            "navy" => Some(Color::rgb(0, 0, 128)),
            "darkblue" => Some(Color::rgb(0, 0, 139)),
            "mediumblue" => Some(Color::rgb(0, 0, 205)),
            "royalblue" => Some(Color::rgb(65, 105, 225)),
            "steelblue" => Some(Color::rgb(70, 130, 180)),
            "dodgerblue" => Some(Color::rgb(30, 144, 255)),
            "deepskyblue" => Some(Color::rgb(0, 191, 255)),
            "cornflowerblue" => Some(Color::rgb(100, 149, 237)),
            "skyblue" => Some(Color::rgb(135, 206, 235)),
            "lightskyblue" => Some(Color::rgb(135, 206, 250)),
            "lightblue" => Some(Color::rgb(173, 216, 230)),
            "powderblue" => Some(Color::rgb(176, 224, 230)),
            "lightsteelblue" => Some(Color::rgb(176, 196, 222)),
            "cadetblue" => Some(Color::rgb(95, 158, 160)),
            "slateblue" => Some(Color::rgb(106, 90, 205)),
            "darkslateblue" => Some(Color::rgb(72, 61, 139)),
            "mediumslateblue" => Some(Color::rgb(123, 104, 238)),

            // Cyans/Teals
            "teal" => Some(Color::rgb(0, 128, 128)),
            "darkcyan" => Some(Color::rgb(0, 139, 139)),
            "lightcyan" => Some(Color::rgb(224, 255, 255)),
            "aquamarine" => Some(Color::rgb(127, 255, 212)),
            "turquoise" => Some(Color::rgb(64, 224, 208)),
            "mediumturquoise" => Some(Color::rgb(72, 209, 204)),
            "darkturquoise" => Some(Color::rgb(0, 206, 209)),
            "paleturquoise" => Some(Color::rgb(175, 238, 238)),

            // Purples
            "purple" => Some(Color::rgb(128, 0, 128)),
            "darkmagenta" => Some(Color::rgb(139, 0, 139)),
            "darkviolet" => Some(Color::rgb(148, 0, 211)),
            "darkorchid" => Some(Color::rgb(153, 50, 204)),
            "mediumorchid" => Some(Color::rgb(186, 85, 211)),
            "orchid" => Some(Color::rgb(218, 112, 214)),
            "violet" => Some(Color::rgb(238, 130, 238)),
            "plum" => Some(Color::rgb(221, 160, 221)),
            "thistle" => Some(Color::rgb(216, 191, 216)),
            "lavender" => Some(Color::rgb(230, 230, 250)),
            "indigo" => Some(Color::rgb(75, 0, 130)),
            "mediumpurple" => Some(Color::rgb(147, 112, 219)),
            "blueviolet" => Some(Color::rgb(138, 43, 226)),

            // Pinks
            "pink" => Some(Color::rgb(255, 192, 203)),
            "lightpink" => Some(Color::rgb(255, 182, 193)),
            "hotpink" => Some(Color::rgb(255, 105, 180)),
            "deeppink" => Some(Color::rgb(255, 20, 147)),
            "mediumvioletred" => Some(Color::rgb(199, 21, 133)),
            "palevioletred" => Some(Color::rgb(219, 112, 147)),

            // Browns
            "brown" => Some(Color::rgb(165, 42, 42)),
            "saddlebrown" => Some(Color::rgb(139, 69, 19)),
            "sienna" => Some(Color::rgb(160, 82, 45)),
            "chocolate" => Some(Color::rgb(210, 105, 30)),
            "peru" => Some(Color::rgb(205, 133, 63)),
            "sandybrown" => Some(Color::rgb(244, 164, 96)),
            "burlywood" => Some(Color::rgb(222, 184, 135)),
            "tan" => Some(Color::rgb(210, 180, 140)),
            "rosybrown" => Some(Color::rgb(188, 143, 143)),

            // Whites
            "snow" => Some(Color::rgb(255, 250, 250)),
            "honeydew" => Some(Color::rgb(240, 255, 240)),
            "mintcream" => Some(Color::rgb(245, 255, 250)),
            "azure" => Some(Color::rgb(240, 255, 255)),
            "aliceblue" => Some(Color::rgb(240, 248, 255)),
            "ghostwhite" => Some(Color::rgb(248, 248, 255)),
            "whitesmoke" => Some(Color::rgb(245, 245, 245)),
            "seashell" => Some(Color::rgb(255, 245, 238)),
            "beige" => Some(Color::rgb(245, 245, 220)),
            "oldlace" => Some(Color::rgb(253, 245, 230)),
            "floralwhite" => Some(Color::rgb(255, 250, 240)),
            "ivory" => Some(Color::rgb(255, 255, 240)),
            "antiquewhite" => Some(Color::rgb(250, 235, 215)),
            "linen" => Some(Color::rgb(250, 240, 230)),
            "lavenderblush" => Some(Color::rgb(255, 240, 245)),
            "mistyrose" => Some(Color::rgb(255, 228, 225)),
            "papayawhip" => Some(Color::rgb(255, 239, 213)),
            "blanchedalmond" => Some(Color::rgb(255, 235, 205)),
            "bisque" => Some(Color::rgb(255, 228, 196)),
            "moccasin" => Some(Color::rgb(255, 228, 181)),
            "navajowhite" => Some(Color::rgb(255, 222, 173)),
            "peachpuff" => Some(Color::rgb(255, 218, 185)),
            "wheat" => Some(Color::rgb(245, 222, 179)),
            "cornsilk" => Some(Color::rgb(255, 248, 220)),

            // Others
            "slategray" | "slategrey" => Some(Color::rgb(112, 128, 144)),
            "lightslategray" | "lightslategrey" => Some(Color::rgb(119, 136, 153)),
            "darkslategray" | "darkslategrey" => Some(Color::rgb(47, 79, 79)),

            // Transparent
            "transparent" => Some(Color::rgba(0, 0, 0, 0)),

            // Current color (marker value)
            "currentcolor" => Some(Color::rgb(0, 0, 0)), // Will need special handling

            _ => None,
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::rgb(0, 0, 0)
    }
}

/// Value parser for converting tokens to CSS values
pub struct ValueParser;

impl ValueParser {
    /// Parse a single token into a CSS value
    pub fn parse_token(token: &Token, location: SourceLocation) -> CssResult<CssValue> {
        match token {
            Token::Ident(name) => {
                // Check if it's a color name
                if let Some(color) = Color::from_name(name) {
                    return Ok(CssValue::Color(color));
                }
                Ok(CssValue::Keyword(name.clone()))
            }
            Token::Hash(value, _) => {
                // Try to parse as color
                if let Some(color) = Color::from_hex(value) {
                    return Ok(CssValue::Color(color));
                }
                Err(CssError::InvalidColor {
                    color: format!("#{}", value),
                    location,
                })
            }
            Token::Number(n) => Ok(CssValue::Number(*n)),
            Token::Percentage(n) => Ok(CssValue::Percentage(*n)),
            Token::Dimension(n, unit) => {
                if let Some(length_unit) = LengthUnit::from_str(unit) {
                    Ok(CssValue::Length(*n, length_unit))
                } else {
                    // Unknown unit - treat as keyword for now
                    Err(CssError::parse_error(format!("Unknown unit: {}", unit), location))
                }
            }
            Token::String(s) => Ok(CssValue::String(s.clone())),
            Token::Url(url) => Ok(CssValue::Url(url.clone())),
            _ => Err(CssError::parse_error(format!("Unexpected token: {:?}", token), location)),
        }
    }

    /// Parse RGB function arguments
    pub fn parse_rgb(args: &[Token], location: SourceLocation) -> CssResult<Color> {
        // Filter out whitespace and commas to get values
        let values: Vec<_> = args.iter()
            .filter(|t| !matches!(t, Token::Whitespace | Token::Comma))
            .collect();

        if values.len() < 3 {
            return Err(CssError::parse_error("rgb() requires 3 arguments", location));
        }

        let r = Self::parse_color_component(values[0], location)?;
        let g = Self::parse_color_component(values[1], location)?;
        let b = Self::parse_color_component(values[2], location)?;

        // Optional alpha
        let a = if values.len() >= 4 {
            Self::parse_alpha_component(values[3], location)?
        } else {
            255
        };

        Ok(Color::rgba(r, g, b, a))
    }

    /// Parse a single color component (0-255 or 0%-100%)
    fn parse_color_component(token: &Token, location: SourceLocation) -> CssResult<u8> {
        match token {
            Token::Number(n) => Ok((*n).clamp(0.0, 255.0) as u8),
            Token::Percentage(p) => Ok((p / 100.0 * 255.0).clamp(0.0, 255.0) as u8),
            _ => Err(CssError::parse_error("Invalid color component", location)),
        }
    }

    /// Parse an alpha component (0-1 or 0%-100%)
    fn parse_alpha_component(token: &Token, location: SourceLocation) -> CssResult<u8> {
        match token {
            Token::Number(n) => Ok((n.clamp(0.0, 1.0) * 255.0) as u8),
            Token::Percentage(p) => Ok((p / 100.0 * 255.0).clamp(0.0, 255.0) as u8),
            _ => Err(CssError::parse_error("Invalid alpha component", location)),
        }
    }

    /// Parse HSL function arguments
    pub fn parse_hsl(args: &[Token], location: SourceLocation) -> CssResult<Color> {
        let values: Vec<_> = args.iter()
            .filter(|t| !matches!(t, Token::Whitespace | Token::Comma))
            .collect();

        if values.len() < 3 {
            return Err(CssError::parse_error("hsl() requires 3 arguments", location));
        }

        let h = match values[0] {
            Token::Number(n) => *n,
            Token::Dimension(n, unit) if unit == "deg" => *n,
            _ => return Err(CssError::parse_error("Invalid hue value", location)),
        };

        let s = match values[1] {
            Token::Percentage(p) => *p / 100.0,
            Token::Number(n) => *n / 100.0,
            _ => return Err(CssError::parse_error("Invalid saturation value", location)),
        };

        let l = match values[2] {
            Token::Percentage(p) => *p / 100.0,
            Token::Number(n) => *n / 100.0,
            _ => return Err(CssError::parse_error("Invalid lightness value", location)),
        };

        let a = if values.len() >= 4 {
            Self::parse_alpha_component(values[3], location)? as f32 / 255.0
        } else {
            1.0
        };

        let (r, g, b) = hsl_to_rgb(h, s, l);
        Ok(Color::rgba(
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (a * 255.0) as u8,
        ))
    }
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let h = (h % 360.0 + 360.0) % 360.0 / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s == 0.0 {
        return (l, l, l);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    (r, g, b)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 { t += 1.0; }
    if t > 1.0 { t -= 1.0; }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_color_3() {
        let color = Color::from_hex("fff").unwrap();
        assert_eq!(color, Color::rgb(255, 255, 255));
    }

    #[test]
    fn test_hex_color_6() {
        let color = Color::from_hex("ff0000").unwrap();
        assert_eq!(color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_hex_color_8() {
        let color = Color::from_hex("ff000080").unwrap();
        assert_eq!(color, Color::rgba(255, 0, 0, 128));
    }

    #[test]
    fn test_named_color() {
        assert_eq!(Color::from_name("red"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(Color::from_name("RED"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(Color::from_name("transparent"), Some(Color::rgba(0, 0, 0, 0)));
    }

    #[test]
    fn test_length_unit_parse() {
        assert_eq!(LengthUnit::from_str("px"), Some(LengthUnit::Px));
        assert_eq!(LengthUnit::from_str("em"), Some(LengthUnit::Em));
        assert_eq!(LengthUnit::from_str("REM"), Some(LengthUnit::Rem));
        assert_eq!(LengthUnit::from_str("vw"), Some(LengthUnit::Vw));
        assert_eq!(LengthUnit::from_str("unknown"), None);
    }

    #[test]
    fn test_hsl_to_rgb() {
        // Red
        let (r, g, b) = hsl_to_rgb(0.0, 1.0, 0.5);
        assert!((r - 1.0).abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!(b.abs() < 0.01);

        // Green
        let (r, g, b) = hsl_to_rgb(120.0, 1.0, 0.5);
        assert!(r.abs() < 0.01);
        assert!((g - 1.0).abs() < 0.01);
        assert!(b.abs() < 0.01);

        // Blue
        let (r, g, b) = hsl_to_rgb(240.0, 1.0, 0.5);
        assert!(r.abs() < 0.01);
        assert!(g.abs() < 0.01);
        assert!((b - 1.0).abs() < 0.01);
    }
}

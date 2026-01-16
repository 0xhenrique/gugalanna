//! CSS Property Definitions
//!
//! Defines CSS properties and their inheritance behavior.

/// Whether a property is inherited by default
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inheritance {
    /// Property is inherited from parent
    Inherited,
    /// Property is not inherited (uses initial value)
    NotInherited,
}

/// Information about a CSS property
#[derive(Debug)]
pub struct PropertyInfo {
    /// The property name
    pub name: &'static str,
    /// Whether this property is inherited
    pub inheritance: Inheritance,
}

impl PropertyInfo {
    const fn new(name: &'static str, inheritance: Inheritance) -> Self {
        Self { name, inheritance }
    }
}

/// Get property information by name
pub fn get_property_info(name: &str) -> Option<PropertyInfo> {
    let inheritance = get_inheritance(name)?;
    // Find the canonical property name from our static list
    let canonical_name = PROPERTY_NAMES.iter()
        .find(|&&n| n.eq_ignore_ascii_case(name))
        .copied()
        .unwrap_or("unknown");
    Some(PropertyInfo {
        name: canonical_name,
        inheritance,
    })
}

/// Check if a property is inherited by default
pub fn get_inheritance(property: &str) -> Option<Inheritance> {
    match property.to_ascii_lowercase().as_str() {
        // Inherited properties (text and font related)
        "color" |
        "font" |
        "font-family" |
        "font-size" |
        "font-style" |
        "font-variant" |
        "font-weight" |
        "letter-spacing" |
        "line-height" |
        "list-style" |
        "list-style-image" |
        "list-style-position" |
        "list-style-type" |
        "text-align" |
        "text-indent" |
        "text-transform" |
        "visibility" |
        "white-space" |
        "word-spacing" |
        "cursor" |
        "direction" |
        "quotes" => Some(Inheritance::Inherited),

        // Not inherited properties (box model, positioning, etc.)
        "display" |
        "position" |
        "top" |
        "right" |
        "bottom" |
        "left" |
        "float" |
        "clear" |
        "z-index" |
        "overflow" |
        "overflow-x" |
        "overflow-y" |
        "width" |
        "height" |
        "min-width" |
        "min-height" |
        "max-width" |
        "max-height" |
        "margin" |
        "margin-top" |
        "margin-right" |
        "margin-bottom" |
        "margin-left" |
        "padding" |
        "padding-top" |
        "padding-right" |
        "padding-bottom" |
        "padding-left" |
        "border" |
        "border-width" |
        "border-style" |
        "border-color" |
        "border-top" |
        "border-right" |
        "border-bottom" |
        "border-left" |
        "border-top-width" |
        "border-right-width" |
        "border-bottom-width" |
        "border-left-width" |
        "border-top-style" |
        "border-right-style" |
        "border-bottom-style" |
        "border-left-style" |
        "border-top-color" |
        "border-right-color" |
        "border-bottom-color" |
        "border-left-color" |
        "background" |
        "background-color" |
        "background-image" |
        "background-repeat" |
        "background-position" |
        "background-attachment" |
        "background-size" |
        "vertical-align" |
        "text-decoration" |
        "text-decoration-color" |
        "text-decoration-line" |
        "text-decoration-style" |
        "box-sizing" |
        "content" |
        "outline" |
        "outline-width" |
        "outline-style" |
        "outline-color" |
        "opacity" |
        "transform" |
        "transition" |
        "animation" |
        "flex" |
        "flex-direction" |
        "flex-wrap" |
        "flex-flow" |
        "flex-grow" |
        "flex-shrink" |
        "flex-basis" |
        "justify-content" |
        "align-items" |
        "align-self" |
        "align-content" |
        "order" |
        "grid" |
        "grid-template" |
        "grid-template-columns" |
        "grid-template-rows" |
        "grid-area" |
        "grid-column" |
        "grid-row" |
        "gap" |
        "row-gap" |
        "column-gap" => Some(Inheritance::NotInherited),

        _ => None,
    }
}

/// Check if a property should be inherited
pub fn is_inherited(property: &str) -> bool {
    matches!(get_inheritance(property), Some(Inheritance::Inherited))
}

/// List of known property names
static PROPERTY_NAMES: &[&str] = &[
    "color",
    "font",
    "font-family",
    "font-size",
    "font-style",
    "font-variant",
    "font-weight",
    "letter-spacing",
    "line-height",
    "list-style",
    "list-style-image",
    "list-style-position",
    "list-style-type",
    "text-align",
    "text-indent",
    "text-transform",
    "visibility",
    "white-space",
    "word-spacing",
    "cursor",
    "direction",
    "quotes",
    "display",
    "position",
    "top",
    "right",
    "bottom",
    "left",
    "float",
    "clear",
    "z-index",
    "overflow",
    "overflow-x",
    "overflow-y",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "margin",
    "margin-top",
    "margin-right",
    "margin-bottom",
    "margin-left",
    "padding",
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
    "border",
    "border-width",
    "border-style",
    "border-color",
    "border-top",
    "border-right",
    "border-bottom",
    "border-left",
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
    "background",
    "background-color",
    "background-image",
    "background-repeat",
    "background-position",
    "background-attachment",
    "background-size",
    "vertical-align",
    "text-decoration",
    "box-sizing",
    "content",
    "outline",
    "outline-width",
    "outline-style",
    "outline-color",
    "opacity",
    "transform",
    "flex",
    "flex-direction",
    "flex-wrap",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inherited_properties() {
        assert!(is_inherited("color"));
        assert!(is_inherited("font-family"));
        assert!(is_inherited("font-size"));
        assert!(is_inherited("line-height"));
        assert!(is_inherited("text-align"));
    }

    #[test]
    fn test_not_inherited_properties() {
        assert!(!is_inherited("display"));
        assert!(!is_inherited("margin"));
        assert!(!is_inherited("padding"));
        assert!(!is_inherited("width"));
        assert!(!is_inherited("background-color"));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(is_inherited("Color"));
        assert!(is_inherited("COLOR"));
        assert!(!is_inherited("Display"));
        assert!(!is_inherited("DISPLAY"));
    }
}

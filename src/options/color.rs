//! Colour values of the style options.

use crate::color::ColorMode;
use crate::ir::Str;
use alloc::string::String;
use core::fmt;

/// A CSS colour, kept exactly as written.
///
/// The text is written into the SVG unchanged (and escaped there), so
/// `"rgb(20,60,160)"` and `"rgb(20, 60, 160)"` stay distinct. Only the empty
/// string is rejected, because it means "not set" (`None`). The CSS colour
/// grammar is left to the consumer: any keyword, `#rrggbb`, `hsl()` or
/// `var()` value passes, and so does text milsymbol.js accepts, whatever
/// characters it holds.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(try_from = "String", into = "String"))]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Color(Str);

/// Why a string is not a [`Color`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ColorError {
    /// The text is empty. Use `None` for "not set".
    Empty,
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColorError::Empty => f.write_str("a colour cannot be empty; use None for not set"),
        }
    }
}

impl core::error::Error for ColorError {}

impl Color {
    /// A colour from CSS text, as a `String` or a `&'static str` (a borrowed
    /// `&String` does not compile; pass `css.clone()`).
    pub fn new(css: impl Into<Str>) -> Result<Self, ColorError> {
        let css = css.into();
        if css.is_empty() {
            Err(ColorError::Empty)
        } else {
            Ok(Color(css))
        }
    }

    /// A colour from constant text, for defaults; the text must not be
    /// empty.
    pub(crate) const fn from_static(css: &'static str) -> Self {
        Color(Str::Borrowed(css))
    }

    /// `rgb(r,g,b)`.
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color(Str::Owned(alloc::format!("rgb({r},{g},{b})")))
    }

    /// The CSS text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn to_str(&self) -> Str {
        self.0.clone()
    }
}

impl AsRef<str> for Color {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<&str> for Color {
    type Error = ColorError;

    fn try_from(css: &str) -> Result<Self, ColorError> {
        Color::new(String::from(css))
    }
}

impl TryFrom<String> for Color {
    type Error = ColorError;

    fn try_from(css: String) -> Result<Self, ColorError> {
        Color::new(css)
    }
}

/// A colour option: one colour, or one colour per affiliation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ColorChoice {
    /// A single colour for every affiliation.
    Uniform(Color),
    /// One colour per affiliation.
    PerAffiliation(ColorMode),
}

impl ColorChoice {
    /// The per-affiliation colours, if this is a per-affiliation choice.
    pub fn as_mode(&self) -> Option<&ColorMode> {
        match self {
            ColorChoice::PerAffiliation(m) => Some(m),
            ColorChoice::Uniform(_) => None,
        }
    }
}

impl From<Color> for ColorChoice {
    fn from(c: Color) -> Self {
        ColorChoice::Uniform(c)
    }
}

impl From<ColorMode> for ColorChoice {
    fn from(m: ColorMode) -> Self {
        ColorChoice::PerAffiliation(m)
    }
}

/// The fill colour mode: a registered mode by name, or a custom one.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ColorModeChoice {
    /// A mode registered with the renderer (`Light`, `Medium`, `Dark`, …).
    Named(Str),
    /// A mode given inline.
    Custom(ColorMode),
}

impl ColorModeChoice {
    /// A mode registered under `name`.
    pub fn named(name: impl Into<Str>) -> Self {
        ColorModeChoice::Named(name.into())
    }
}

impl From<ColorMode> for ColorModeChoice {
    fn from(m: ColorMode) -> Self {
        ColorModeChoice::Custom(m)
    }
}

impl From<Color> for String {
    fn from(c: Color) -> String {
        c.0.into_owned()
    }
}

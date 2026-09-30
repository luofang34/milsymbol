//! Assigning options by name, e.g. `size` or `uniqueDesignation`.

use super::{Color, ColorChoice, ColorModeChoice, SymbolOptions, TextField};
use crate::color::ColorMode;
use alloc::string::String;
use core::fmt;

/// A value for [`SymbolOptions::set`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum OptionValue {
    /// A string.
    Str(String),
    /// A number.
    Num(f64),
    /// A boolean (`infoOutlineWidth: false` is accepted as "unset").
    Bool(bool),
    /// One colour per affiliation.
    Colors(ColorMode),
}

impl From<&str> for OptionValue {
    fn from(s: &str) -> Self {
        OptionValue::Str(String::from(s))
    }
}

impl From<f64> for OptionValue {
    fn from(v: f64) -> Self {
        OptionValue::Num(v)
    }
}

impl From<bool> for OptionValue {
    fn from(v: bool) -> Self {
        OptionValue::Bool(v)
    }
}

/// Why [`SymbolOptions::set`] rejected an option.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OptionError {
    /// No option has this name. Custom text fields (e.g. for label
    /// overrides) are set with [`SymbolOptions::set_text`].
    Unknown {
        /// The option name.
        key: String,
    },
    /// The value has the wrong type or is not one of the accepted values.
    Invalid {
        /// The option name.
        key: String,
        /// What the option accepts.
        expected: &'static str,
    },
}

impl OptionError {
    /// The option name.
    pub fn key(&self) -> &str {
        match self {
            OptionError::Unknown { key } | OptionError::Invalid { key, .. } => key,
        }
    }
}

impl fmt::Display for OptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OptionError::Unknown { key } => write!(f, "unknown option {key:?}"),
            OptionError::Invalid { key, expected } => {
                write!(f, "option {key:?} expects {expected}")
            }
        }
    }
}

impl core::error::Error for OptionError {}

fn err<T>(key: &str, expected: &'static str) -> Result<T, OptionError> {
    Err(OptionError::Invalid {
        key: String::from(key),
        expected,
    })
}

fn num(key: &str, v: OptionValue) -> Result<f64, OptionError> {
    match v {
        OptionValue::Num(n) => Ok(n),
        _ => err(key, "a number"),
    }
}

fn boolean(key: &str, v: OptionValue) -> Result<bool, OptionError> {
    match v {
        OptionValue::Bool(b) => Ok(b),
        _ => err(key, "a boolean"),
    }
}

fn string(key: &str, v: OptionValue) -> Result<String, OptionError> {
    match v {
        OptionValue::Str(s) => Ok(s),
        _ => err(key, "a string"),
    }
}

fn css(s: String) -> Result<Option<Color>, OptionError> {
    if s.is_empty() {
        return Ok(None);
    }
    Ok(Color::new(s).ok())
}

fn color(key: &str, v: OptionValue) -> Result<Option<ColorChoice>, OptionError> {
    match v {
        OptionValue::Str(s) => Ok(css(s)?.map(ColorChoice::Uniform)),
        OptionValue::Colors(m) => Ok(Some(ColorChoice::PerAffiliation(m))),
        _ => err(key, "a colour string or per-affiliation colours"),
    }
}

fn color_mode(key: &str, v: OptionValue) -> Result<ColorModeChoice, OptionError> {
    match v {
        OptionValue::Str(s) => Ok(ColorModeChoice::Named(s.into())),
        OptionValue::Colors(m) => Ok(ColorModeChoice::Custom(m)),
        _ => err(key, "a colour mode name or per-affiliation colours"),
    }
}

impl SymbolOptions {
    /// Sets an option or style value by name (e.g. `size`, `colorMode`,
    /// `uniqueDesignation`).
    ///
    /// Unknown names are rejected; custom text fields (label overrides use
    /// keys such as `dtg1`) are set with [`SymbolOptions::set_text`]. `sidc`
    /// is not an option here; pass it to
    /// [`Renderer::symbol`](crate::Renderer::symbol).
    ///
    /// ```
    /// use milsymbol::options::{OptionError, SymbolOptions};
    ///
    /// let mut o = SymbolOptions::default();
    /// o.set("size", 50.0)?.set("uniqueDesignation", "1-66")?;
    /// assert_eq!(o.style.size, 50.0);
    /// assert!(matches!(o.set("size", "big"), Err(OptionError::Invalid { .. })));
    /// assert!(matches!(o.set("sise", 50.0), Err(OptionError::Unknown { .. })));
    /// # Ok::<(), OptionError>(())
    /// ```
    pub fn set(
        &mut self,
        key: &str,
        value: impl Into<OptionValue>,
    ) -> Result<&mut Self, OptionError> {
        let Some(v) = self.set_style(key, value.into())? else {
            return Ok(self);
        };
        match key {
            "sidc" | "SIDC" => return err(key, "to be passed to Renderer::symbol"),
            "direction" => {
                self.direction = match v {
                    OptionValue::Num(n) => Some(n),
                    OptionValue::Str(s) if s.is_empty() => None,
                    _ => return err(key, "a number"),
                }
            }
            "speedLeader" => self.speed_leader = Some(num(key, v)?),
            "stack" => self.stack = Some(num(key, v)?),
            "country_flag" => self.country_flag = Some(string(key, v)?),
            "full_frame_flag" => self.full_frame_flag = Some(boolean(key, v)?),
            "signature" => self.signature = Some(string(key, v)?),
            _ if TextField::STANDARD.iter().any(|f| f.name() == key) => {
                self.text.insert(String::from(key), string(key, v)?);
            }
            _ => {
                return Err(OptionError::Unknown {
                    key: String::from(key),
                });
            }
        }
        Ok(self)
    }

    /// Sets a style option; hands `v` back when `key` is not one.
    fn set_style(&mut self, key: &str, v: OptionValue) -> Result<Option<OptionValue>, OptionError> {
        let st = &mut self.style;
        match key {
            "alternateMedal" => st.alternate_medal = boolean(key, v)?,
            "civilianColor" => st.civilian_color = boolean(key, v)?,
            "colorMode" => st.color_mode = color_mode(key, v)?,
            "fill" => st.fill = boolean(key, v)?,
            "fillColor" => st.fill_color = css(string(key, v)?)?,
            "fillOpacity" => st.fill_opacity = num(key, v)?,
            "fontfamily" => st.font_family = string(key, v)?.into(),
            "frame" => st.frame = boolean(key, v)?,
            "frameColor" => st.frame_color = color(key, v)?,
            "hqStaffLength" => st.hq_staff_length = Some(num(key, v)?),
            "icon" => st.icon = boolean(key, v)?,
            "iconTextUsesFontFamily" => st.icon_text_uses_font_family = boolean(key, v)?,
            "iconColor" => st.icon_color = color(key, v)?,
            "infoBackground" => st.info_background = color(key, v)?,
            "infoBackgroundFrame" => st.info_background_frame = color(key, v)?,
            "infoColor" => st.info_color = color(key, v)?,
            "infoFields" => st.info_fields = boolean(key, v)?,
            "infoOutlineColor" => st.info_outline_color = css(string(key, v)?)?,
            "infoOutlineWidth" => {
                st.info_outline_width = match v {
                    OptionValue::Num(n) => Some(n),
                    OptionValue::Bool(false) => None,
                    _ => return err(key, "a number or false"),
                }
            }
            "infoSize" => st.info_size = num(key, v)?,
            "monoColor" => st.mono_color = css(string(key, v)?)?,
            "outlineColor" => st.outline_color = color(key, v)?,
            "outlineWidth" => st.outline_width = num(key, v)?,
            "padding" => st.padding = num(key, v)?,
            "simpleStatusModifier" => st.simple_status_modifier = boolean(key, v)?,
            "size" => st.size = num(key, v)?,
            "square" => st.square = boolean(key, v)?,
            "standard" => {
                st.standard = match string(key, v)?.as_str() {
                    "" => None,
                    "2525" => Some(crate::Standard::Mil2525),
                    "APP6" => Some(crate::Standard::App6),
                    _ => return err(key, "\"2525\", \"APP6\" or \"\""),
                }
            }
            "strokeWidth" => st.stroke_width = num(key, v)?,
            "styleFill" => st.style_fill = boolean(key, v)?,
            _ => return Ok(Some(v)),
        }
        Ok(None)
    }
}

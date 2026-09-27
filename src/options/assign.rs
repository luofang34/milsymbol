//! Assigning options by upstream name (the shape of milsymbol.js
//! `new ms.Symbol(sidc, { size: 30, uniqueDesignation: "A" })`).

use super::{StyleColor, SymbolOptions};
use crate::color::ColorMode;
use alloc::string::String;
use core::fmt;

/// A value for [`SymbolOptions::set`].
#[derive(Debug, Clone, PartialEq)]
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

/// An option name and value combination milsymbol.js does not support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionError {
    /// The option name.
    pub key: String,
    /// What the option accepts.
    pub expected: &'static str,
}

impl fmt::Display for OptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "option {:?} expects {}", self.key, self.expected)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OptionError {}

fn err<T>(key: &str, expected: &'static str) -> Result<T, OptionError> {
    Err(OptionError {
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

fn color(key: &str, v: OptionValue) -> Result<StyleColor, OptionError> {
    match v {
        OptionValue::Str(s) => Ok(StyleColor::Str(s)),
        OptionValue::Colors(m) => Ok(StyleColor::PerAffiliation(m)),
        _ => err(key, "a colour string or per-affiliation colours"),
    }
}

impl SymbolOptions {
    /// Sets an option or style value by its milsymbol.js name.
    ///
    /// Unknown string-valued keys are kept as extra text fields (label
    /// overrides use keys such as `dtg1`). `sidc` is not an option here; pass
    /// it to [`Renderer::symbol`](crate::Renderer::symbol).
    pub fn set(
        &mut self,
        key: &str,
        value: impl Into<OptionValue>,
    ) -> Result<&mut Self, OptionError> {
        let v = value.into();
        if self.set_style(key, v.clone())? {
            return Ok(self);
        }
        match key {
            "sidc" | "SIDC" => return err(key, "to be passed to Renderer::symbol"),
            "direction" => {
                self.direction = match v {
                    OptionValue::Num(n) => Some(n),
                    OptionValue::Str(s) if s.is_empty() => None,
                    _ => return err(key, "a number"),
                }
            }
            "speedLeader" => self.speed_leader = num(key, v)?,
            "stack" => self.stack = Some(num(key, v)?),
            "country_flag" => self.country_flag = Some(string(key, v)?),
            "full_frame_flag" => self.full_frame_flag = Some(boolean(key, v)?),
            "signature" => self.signature = Some(string(key, v)?),
            _ => {
                self.text.insert(String::from(key), string(key, v)?);
            }
        }
        Ok(self)
    }

    fn set_style(&mut self, key: &str, v: OptionValue) -> Result<bool, OptionError> {
        let st = &mut self.style;
        match key {
            "alternateMedal" => st.alternate_medal = boolean(key, v)?,
            "civilianColor" => st.civilian_color = boolean(key, v)?,
            "colorMode" => st.color_mode = color(key, v)?,
            "fill" => st.fill = boolean(key, v)?,
            "fillColor" => st.fill_color = string(key, v)?,
            "fillOpacity" => st.fill_opacity = num(key, v)?,
            "fontfamily" => st.font_family = string(key, v)?,
            "frame" => st.frame = boolean(key, v)?,
            "frameColor" => st.frame_color = color(key, v)?,
            "hqStaffLength" => st.hq_staff_length = num(key, v)?,
            "icon" => st.icon = boolean(key, v)?,
            "iconColor" => st.icon_color = color(key, v)?,
            "infoBackground" => st.info_background = color(key, v)?,
            "infoBackgroundFrame" => st.info_background_frame = color(key, v)?,
            "infoColor" => st.info_color = color(key, v)?,
            "infoFields" => st.info_fields = boolean(key, v)?,
            "infoOutlineColor" => st.info_outline_color = string(key, v)?,
            "infoOutlineWidth" => {
                st.info_outline_width = match v {
                    OptionValue::Num(n) => Some(n),
                    OptionValue::Bool(false) => None,
                    _ => return err(key, "a number or false"),
                }
            }
            "infoSize" => st.info_size = num(key, v)?,
            "monoColor" => st.mono_color = string(key, v)?,
            "outlineColor" => st.outline_color = color(key, v)?,
            "outlineWidth" => st.outline_width = num(key, v)?,
            "padding" => st.padding = num(key, v)?,
            "simpleStatusModifier" => st.simple_status_modifier = boolean(key, v)?,
            "size" => st.size = num(key, v)?,
            "square" => st.square = boolean(key, v)?,
            "standard" => st.standard = string(key, v)?,
            "strokeWidth" => st.stroke_width = num(key, v)?,
            "styleFill" => st.style_fill = boolean(key, v)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

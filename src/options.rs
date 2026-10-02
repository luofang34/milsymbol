//! Symbol options: text amplifiers, modifiers and style.
//!
//! Options are typed fields; [`TextField`] names the text amplifiers.
//! [`SymbolOptions::set`] also accepts milsymbol.js option names.

use crate::ir::Str;
use alloc::collections::BTreeMap;
use alloc::string::String;

mod assign;
pub use assign::{OptionError, OptionValue};

mod color;
mod text_field;
mod validate;

pub use color::{Color, ColorChoice, ColorError, ColorModeChoice};
pub(crate) use text_field::field;
pub use text_field::{CustomField, TextField};

/// Style options.
///
/// Start from [`Style::default`] and assign fields, or set options by their
/// milsymbol.js names with [`SymbolOptions::set`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Style {
    /// Use the alternate MEDAL icons for sea mines.
    pub alternate_medal: bool,
    /// Use civilian purple for civilian symbols.
    pub civilian_color: bool,
    /// Fill colour mode: a registered mode name or a custom mode.
    pub color_mode: ColorModeChoice,
    /// Fill the frame.
    pub fill: bool,
    /// Frame fill colour override.
    pub fill_color: Option<Color>,
    /// Frame fill opacity.
    pub fill_opacity: f64,
    /// Font family for generated text. Built-in icon text uses it when
    /// `icon_text_uses_font_family` is enabled.
    pub font_family: Str,
    /// Draw the frame.
    pub frame: bool,
    /// Frame colour override (per affiliation).
    pub frame_color: Option<ColorChoice>,
    /// Headquarters staff length; `None` uses the renderer default.
    pub hq_staff_length: Option<f64>,
    /// Draw the icon.
    pub icon: bool,
    /// Use `font_family` for built-in icon text instead of its template font.
    /// Defaults to `false` to preserve milsymbol.js SVG output.
    pub icon_text_uses_font_family: bool,
    /// Icon colour override (per affiliation).
    pub icon_color: Option<ColorChoice>,
    /// Background behind the information fields.
    pub info_background: Option<ColorChoice>,
    /// Frame of the information-field background.
    pub info_background_frame: Option<ColorChoice>,
    /// Information-field text colour.
    pub info_color: Option<ColorChoice>,
    /// Draw information fields.
    pub info_fields: bool,
    /// Information-field outline colour; `None` follows `outline_color`.
    pub info_outline_color: Option<Color>,
    /// Information-field outline width; `None` follows `outline_width`.
    pub info_outline_width: Option<f64>,
    /// Information-field font size.
    pub info_size: f64,
    /// Monochrome colour; `None` for full colour.
    pub mono_color: Option<Color>,
    /// Outline colour.
    pub outline_color: Option<ColorChoice>,
    /// Outline width; `0` disables the outline.
    pub outline_width: f64,
    /// Extra padding around the symbol.
    pub padding: f64,
    /// Always use the simple (slash) status modifiers.
    pub simple_status_modifier: bool,
    /// Symbol size (the frame's L dimension).
    pub size: f64,
    /// Make the symbol square around its anchor.
    pub square: bool,
    /// Standard for this symbol; `None` uses the renderer default.
    pub standard: Option<crate::Standard>,
    /// Frame stroke width.
    pub stroke_width: f64,
    /// Replace style-fillable fills with translucent white.
    pub style_fill: bool,
}

const DEFAULT_OUTLINE: &str = "rgb(239, 239, 239)";

impl Default for Style {
    fn default() -> Self {
        Style {
            alternate_medal: false,
            civilian_color: true,
            color_mode: ColorModeChoice::Named(Str::Borrowed("Light")),
            fill: true,
            fill_color: None,
            fill_opacity: 1.0,
            font_family: Str::Borrowed("Arial"),
            frame: true,
            frame_color: None,
            hq_staff_length: None,
            icon: true,
            icon_text_uses_font_family: false,
            icon_color: None,
            info_background: None,
            info_background_frame: None,
            info_color: None,
            info_fields: true,
            info_outline_color: Some(Color::from_static(DEFAULT_OUTLINE)),
            info_outline_width: None,
            info_size: 40.0,
            mono_color: None,
            outline_color: Some(ColorChoice::Uniform(Color::from_static(DEFAULT_OUTLINE))),
            outline_width: 0.0,
            padding: 0.0,
            simple_status_modifier: false,
            size: 100.0,
            square: false,
            standard: None,
            stroke_width: 4.0,
            style_fill: false,
        }
    }
}

/// Non-style symbol options.
///
/// Start from [`SymbolOptions::default`] and assign fields, or set options
/// by their milsymbol.js names with [`SymbolOptions::set`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct SymbolOptions {
    /// Text amplifiers that were set, keyed by option name. Read and write
    /// them with [`SymbolOptions::text`] and [`SymbolOptions::set_text`].
    pub(crate) text: BTreeMap<String, String>,
    /// Field Q: direction of movement in degrees.
    pub direction: Option<f64>,
    /// Speed leader length in pixels; `None` draws a direction arrow instead.
    pub speed_leader: Option<f64>,
    /// Number of stacked frames drawn behind the symbol.
    pub stack: Option<f64>,
    /// Reserves space for a country flag in the right-hand fields.
    pub country_flag: Option<String>,
    /// Suppresses the flag space for full-frame friendly ground flags.
    pub full_frame_flag: Option<bool>,
    /// Signature amplifier; `"!"` reserves extra space.
    pub signature: Option<String>,
    /// The symbol style.
    pub style: Style,
}

impl SymbolOptions {
    /// Value of a text field; a field that was not set reads as empty.
    pub fn text(&self, field: &TextField) -> &str {
        self.text_named(field.name())
    }

    /// Value of the text field with this option name.
    pub fn text_named(&self, name: &str) -> &str {
        self.text.get(name).map_or("", String::as_str)
    }

    /// Draws all text in `family`: information fields and the text inside
    /// built-in icons. The icons keep their template font unless
    /// [`Style::icon_text_uses_font_family`] is set, which this does.
    pub fn set_font(&mut self, family: impl Into<Str>) -> &mut Self {
        self.style.font_family = family.into();
        self.style.icon_text_uses_font_family = true;
        self
    }

    /// Sets a text field.
    pub fn set_text(&mut self, field: impl Into<TextField>, value: impl Into<String>) -> &mut Self {
        self.text
            .insert(String::from(field.into().name()), value.into());
        self
    }

    /// The fields that were set, ordered by option name.
    pub fn text_fields(&self) -> impl Iterator<Item = (TextField, &str)> {
        self.text
            .iter()
            .map(|(name, value)| (TextField::from_name(name), value.as_str()))
    }

    /// The speed leader length, or `0` when a direction arrow is drawn.
    pub(crate) fn speed_leader_px(&self) -> f64 {
        self.speed_leader.unwrap_or(0.0)
    }
}

impl Style {
    /// The headquarters staff length override; `None` when the renderer's
    /// default applies.
    pub(crate) fn hq_staff_length_override(&self) -> Option<f64> {
        self.hq_staff_length.filter(|v| *v != 0.0 && !v.is_nan())
    }

    /// The monochrome colour text, empty for full colour.
    pub(crate) fn mono_color_str(&self) -> &str {
        self.mono_color.as_ref().map_or("", Color::as_str)
    }
}

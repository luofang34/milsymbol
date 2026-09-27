//! Symbol options: text amplifiers, modifiers and style.
//!
//! Field names follow upstream `SymbolOptions` so options translate directly
//! from milsymbol.js code; [`field`] lists the text amplifier keys.

use crate::color::ColorMode;
use alloc::collections::BTreeMap;
use alloc::string::String;

mod assign;
pub use assign::{OptionError, OptionValue};

/// Upstream option names of the text amplifier fields present on every symbol.
pub mod field {
    /// Field C.
    pub const QUANTITY: &str = "quantity";
    /// Field F.
    pub const REINFORCED_REDUCED: &str = "reinforcedReduced";
    /// Field G.
    pub const STAFF_COMMENTS: &str = "staffComments";
    /// Field H.
    pub const ADDITIONAL_INFORMATION: &str = "additionalInformation";
    /// Field J.
    pub const EVALUATION_RATING: &str = "evaluationRating";
    /// Field K.
    pub const COMBAT_EFFECTIVENESS: &str = "combatEffectiveness";
    /// Field L.
    pub const SIGNATURE_EQUIPMENT: &str = "signatureEquipment";
    /// Field M.
    pub const HIGHER_FORMATION: &str = "higherFormation";
    /// Field N.
    pub const HOSTILE: &str = "hostile";
    /// Field P.
    pub const IFF_SIF: &str = "iffSif";
    /// Field R2.
    pub const SIGINT: &str = "sigint";
    /// Field T.
    pub const UNIQUE_DESIGNATION: &str = "uniqueDesignation";
    /// Field V.
    pub const TYPE: &str = "type";
    /// Field W.
    pub const DTG: &str = "dtg";
    /// Field X.
    pub const ALTITUDE_DEPTH: &str = "altitudeDepth";
    /// Field Y.
    pub const LOCATION: &str = "location";
    /// Field Z.
    pub const SPEED: &str = "speed";
    /// Field AA.
    pub const SPECIAL_HEADQUARTERS: &str = "specialHeadquarters";
    /// Field AC.
    pub const COUNTRY: &str = "country";
    /// Field AD.
    pub const PLATFORM_TYPE: &str = "platformType";
    /// Field AE.
    pub const EQUIPMENT_TEARDOWN_TIME: &str = "equipmentTeardownTime";
    /// Field AF.
    pub const COMMON_IDENTIFIER: &str = "commonIdentifier";
    /// Field AG.
    pub const AUXILIARY_EQUIPMENT_INDICATOR: &str = "auxiliaryEquipmentIndicator";
    /// Field AH.
    pub const HEADQUARTERS_ELEMENT: &str = "headquartersElement";
    /// Field AI.
    pub const INSTALLATION_COMPOSITION: &str = "installationComposition";
    /// Field AO.
    pub const ENGAGEMENT_BAR: &str = "engagementBar";
    /// Engagement bar type: `TARGET`, `NON-TARGET` or `EXPIRED`.
    pub const ENGAGEMENT_TYPE: &str = "engagementType";
    /// Field AQ.
    pub const GUARDED_UNIT: &str = "guardedUnit";
    /// Field AR.
    pub const SPECIAL_DESIGNATOR: &str = "specialDesignator";

    /// All default text fields, in upstream declaration order.
    pub const DEFAULTS: [&str; 29] = [
        QUANTITY,
        REINFORCED_REDUCED,
        STAFF_COMMENTS,
        ADDITIONAL_INFORMATION,
        EVALUATION_RATING,
        COMBAT_EFFECTIVENESS,
        SIGNATURE_EQUIPMENT,
        HIGHER_FORMATION,
        HOSTILE,
        IFF_SIF,
        SIGINT,
        UNIQUE_DESIGNATION,
        TYPE,
        DTG,
        ALTITUDE_DEPTH,
        LOCATION,
        SPEED,
        SPECIAL_HEADQUARTERS,
        COUNTRY,
        PLATFORM_TYPE,
        EQUIPMENT_TEARDOWN_TIME,
        COMMON_IDENTIFIER,
        AUXILIARY_EQUIPMENT_INDICATOR,
        HEADQUARTERS_ELEMENT,
        INSTALLATION_COMPOSITION,
        ENGAGEMENT_BAR,
        ENGAGEMENT_TYPE,
        GUARDED_UNIT,
        SPECIAL_DESIGNATOR,
    ];
}

/// A style colour: a CSS string, or one colour per affiliation.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleColor {
    /// A single colour (the empty string means "not set").
    Str(String),
    /// One colour per affiliation.
    PerAffiliation(ColorMode),
}

impl StyleColor {
    /// The unset value (`""`).
    pub fn unset() -> Self {
        StyleColor::Str(String::new())
    }

    /// JavaScript truthiness of the style value.
    pub fn is_set(&self) -> bool {
        match self {
            StyleColor::Str(s) => !s.is_empty(),
            StyleColor::PerAffiliation(_) => true,
        }
    }

    /// The per-affiliation object, if any (upstream `typeof x === "object"`).
    pub fn as_mode(&self) -> Option<&ColorMode> {
        match self {
            StyleColor::PerAffiliation(m) => Some(m),
            StyleColor::Str(_) => None,
        }
    }
}

impl From<&str> for StyleColor {
    fn from(s: &str) -> Self {
        StyleColor::Str(String::from(s))
    }
}

/// Style options (upstream `symbol.style`).
#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    /// Use the alternate MEDAL icons for sea mines.
    pub alternate_medal: bool,
    /// Use civilian purple for civilian symbols.
    pub civilian_color: bool,
    /// Fill colour mode: a registered mode name or a custom mode.
    pub color_mode: StyleColor,
    /// Fill the frame.
    pub fill: bool,
    /// Frame fill colour override.
    pub fill_color: String,
    /// Frame fill opacity.
    pub fill_opacity: f64,
    /// Font family for text.
    pub font_family: String,
    /// Draw the frame.
    pub frame: bool,
    /// Frame colour override (per affiliation).
    pub frame_color: StyleColor,
    /// Headquarters staff length; `0` uses the renderer default.
    pub hq_staff_length: f64,
    /// Draw the icon.
    pub icon: bool,
    /// Icon colour override (per affiliation).
    pub icon_color: StyleColor,
    /// Background behind the information fields.
    pub info_background: StyleColor,
    /// Frame of the information-field background.
    pub info_background_frame: StyleColor,
    /// Information-field text colour.
    pub info_color: StyleColor,
    /// Draw information fields.
    pub info_fields: bool,
    /// Information-field outline colour.
    pub info_outline_color: String,
    /// Information-field outline width; `None` follows `outline_width`.
    pub info_outline_width: Option<f64>,
    /// Information-field font size.
    pub info_size: f64,
    /// Monochrome colour; empty for full colour.
    pub mono_color: String,
    /// Outline colour.
    pub outline_color: StyleColor,
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
    /// Standard override: `""`, `"2525"` or `"APP6"`.
    pub standard: String,
    /// Frame stroke width.
    pub stroke_width: f64,
    /// Replace style-fillable fills with translucent white.
    pub style_fill: bool,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            alternate_medal: false,
            civilian_color: true,
            color_mode: StyleColor::Str(String::from("Light")),
            fill: true,
            fill_color: String::new(),
            fill_opacity: 1.0,
            font_family: String::from("Arial"),
            frame: true,
            frame_color: StyleColor::unset(),
            hq_staff_length: 0.0,
            icon: true,
            icon_color: StyleColor::unset(),
            info_background: StyleColor::unset(),
            info_background_frame: StyleColor::unset(),
            info_color: StyleColor::unset(),
            info_fields: true,
            info_outline_color: String::from("rgb(239, 239, 239)"),
            info_outline_width: None,
            info_size: 40.0,
            mono_color: String::new(),
            outline_color: StyleColor::Str(String::from("rgb(239, 239, 239)")),
            outline_width: 0.0,
            padding: 0.0,
            simple_status_modifier: false,
            size: 100.0,
            square: false,
            standard: String::new(),
            stroke_width: 4.0,
            style_fill: false,
        }
    }
}

/// Non-style symbol options (upstream `symbol.options`).
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolOptions {
    /// Text amplifiers keyed by upstream option name. Holds every
    /// [`field::DEFAULTS`] key plus any extra keys (e.g. `dtg1`) used by
    /// label overrides.
    pub text: BTreeMap<String, String>,
    /// Field Q: direction of movement in degrees.
    pub direction: Option<f64>,
    /// Speed leader length in pixels; `0` draws a direction arrow instead.
    pub speed_leader: f64,
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

impl Default for SymbolOptions {
    fn default() -> Self {
        let mut text = BTreeMap::new();
        for k in field::DEFAULTS {
            text.insert(String::from(k), String::new());
        }
        SymbolOptions {
            text,
            direction: None,
            speed_leader: 0.0,
            stack: None,
            country_flag: None,
            full_frame_flag: None,
            signature: None,
            style: Style::default(),
        }
    }
}

impl SymbolOptions {
    /// Value of a text field; unknown keys read as empty.
    pub fn text(&self, key: &str) -> &str {
        self.text.get(key).map_or("", String::as_str)
    }

    /// Sets a text field (any key; see [`field`]).
    pub fn set_text(&mut self, key: &str, value: impl Into<String>) -> &mut Self {
        self.text.insert(String::from(key), value.into());
        self
    }
}

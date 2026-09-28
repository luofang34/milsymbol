//! Label overrides: fixed text-field placements for specific SIDCs
//! (mostly tactical points), and upstream's text width estimate.

use crate::generated::misc;
use alloc::borrow::Cow;

/// Placement of one text field.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Label {
    /// Anchor x.
    pub x: Option<f64>,
    /// Baseline y.
    pub y: Option<f64>,
    /// Font size.
    pub font_size: Option<f64>,
    /// Text anchor (`start`, `middle`, `end`).
    pub anchor: Option<Cow<'static, str>>,
    /// Font weight.
    pub weight: Option<Cow<'static, str>>,
    /// Dominant baseline.
    pub baseline: Option<Cow<'static, str>>,
    /// Text fill colour, replacing the information-field colour.
    pub fill: Option<Cow<'static, str>>,
    /// Stroke flag (`false` disables the stroke).
    pub stroke: Option<bool>,
}

/// Placements of one option field.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct LabelField {
    /// Upstream option name, e.g. `uniqueDesignation` or `dtg1`.
    pub field: Cow<'static, str>,
    /// Whether upstream declares the placements as an array.
    pub is_array: bool,
    /// Placements; the field text is drawn at each.
    pub labels: Cow<'static, [Label]>,
}

impl Label {
    /// A placement at `(x, y)` with a font size of 12 and `start` anchoring.
    pub fn at(x: f64, y: f64) -> Self {
        Label {
            x: Some(x),
            y: Some(y),
            font_size: Some(12.0),
            anchor: Some(Cow::Borrowed("start")),
            ..Label::default()
        }
    }
}

impl LabelField {
    /// Placements of the option `field`.
    pub fn new(
        field: impl Into<Cow<'static, str>>,
        labels: impl Into<Cow<'static, [Label]>>,
    ) -> Self {
        let labels = labels.into();
        LabelField {
            field: field.into(),
            is_array: labels.len() > 1,
            labels,
        }
    }
}

/// Built-in label overrides for a letter-SIDC generic code or numeric entity.
pub(crate) fn builtin(number_sidc: bool, key: &str) -> Option<&'static [LabelField]> {
    let table = if number_sidc {
        misc::NUMBER_LABELS
    } else {
        misc::LETTER_LABELS
    };
    table
        .binary_search_by(|(k, _)| k.as_bytes().cmp(key.as_bytes()))
        .ok()
        .and_then(|i| table.get(i))
        .map(|(_, v)| *v)
}

/// Upstream `strWidth`: estimated width of `s` at `font_size`, plus
/// `space` (the gap between icon and text).
pub(crate) fn str_width(s: &str, font_size: f64, space: f64) -> f64 {
    if s.is_empty() {
        return 0.0;
    }
    let mut w = 0.0;
    for unit in s.encode_utf16() {
        let cw = misc::CHAR_WIDTHS
            .binary_search_by_key(&unit, |&(c, _)| c)
            .ok()
            .and_then(|i| misc::CHAR_WIDTHS.get(i))
            .map_or(28.5, |&(_, w)| w);
        w += (font_size / 30.0) * cw;
    }
    w + space
}

//! Codes and names mechanically extracted from the pinned standards.

use crate::generated::standards::ENTRIES;

/// The part of a numeric SIDC described by a catalog entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum CodeKind {
    /// Six-digit entity/type/subtype code.
    Entity,
    /// Sector 1 modifier, including the common modifier-set digit.
    Modifier1,
    /// Sector 2 modifier, including the common modifier-set digit.
    Modifier2,
}

/// Whether a code denotes a single-symbol graphic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Scope {
    /// A drawable icon or modifier.
    Point,
    /// An unspecified code with no icon.
    Empty,
    /// A heading in the standard's hierarchy, with no associated graphic.
    Hierarchy,
    /// A line or area requiring geographic control points.
    Multipoint,
}

/// Citation to a row of the pinned source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    /// Document identifier in `tools/standards/pin.json`.
    pub document: &'static str,
    /// Printed page number, not the PDF page index.
    pub page: u16,
    /// Table identifier.
    pub table: &'static str,
}

/// A code and its meaning in one SIDC edition.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// SIDC version (`15` or `16`).
    pub version: u8,
    /// Symbol set, or `"common"` for common modifiers.
    pub symbol_set: &'static str,
    /// Entity or modifier sector.
    pub kind: CodeKind,
    /// Six digits for entities, three for modifiers.
    pub code: &'static str,
    /// Name in the code table.
    pub name: &'static str,
    /// Drawing scope.
    pub scope: Scope,
    /// Source of the code's meaning.
    pub source: Source,
    pub(crate) graphic: Option<usize>,
}

impl Entry {
    /// Source of the converted geometry, when the icon comes from a pinned PDF.
    pub fn drawing_source(&self) -> Option<Source> {
        Some(
            crate::generated::standards::GRAPHICS
                .get(self.graphic?)?
                .source,
        )
    }

    /// Input drawn inside this graphic, such as `altitudeDepth` or `pressure`.
    pub fn input_field(&self) -> Option<&'static str> {
        Some(
            crate::generated::standards::GRAPHICS
                .get(self.graphic?)?
                .input
                .as_ref()?
                .field,
        )
    }
}

/// All codes from the pinned document for `version`, including hierarchy
/// headings and explicitly excluded multipoint graphics.
pub fn entries(version: u8) -> impl Iterator<Item = &'static Entry> + Clone {
    ENTRIES.iter().filter(move |entry| entry.version == version)
}

/// Looks up the meaning of a complete entity or modifier code.
pub fn entry(version: u8, symbol_set: &str, kind: CodeKind, code: &str) -> Option<&'static Entry> {
    let symbol_set = if kind != CodeKind::Entity && !code.starts_with('0') {
        "common"
    } else if symbol_set == "64" && kind != CodeKind::Entity {
        "60"
    } else {
        symbol_set
    };
    let key = (version, symbol_set, kind, code);
    ENTRIES
        .binary_search_by_key(&key, |e| (e.version, e.symbol_set, e.kind, e.code))
        .ok()
        .and_then(|i| ENTRIES.get(i))
}

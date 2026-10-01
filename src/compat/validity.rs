//! The verdict of milsymbol.js `isValid()`.

use alloc::vec::Vec;

/// Why milsymbol.js `isValid()` rejects a symbol.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UpstreamIssue {
    /// The standard identity is not recognised.
    UnknownAffiliation,
    /// The battle dimension or symbol set is not recognised.
    UnknownDimension,
    /// The drawn icon does not exist; a hidden icon counts as found.
    UnknownIcon,
    /// The echelon/mobility amplifier code is not recognised.
    UnknownAmplifier,
    /// An icon references a part that does not exist.
    MissingInstruction,
    /// A text or attribute contains `null`, or a coordinate is not finite.
    /// milsymbol.js counts this as invalid even when the SIDC is fine.
    NullInDrawing,
}

/// The verdict of milsymbol.js `isValid()`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UpstreamValidity {
    /// Every reason `isValid()` is false; empty for a valid symbol.
    pub issues: Vec<UpstreamIssue>,
}

impl UpstreamValidity {
    /// Whether there are no issues.
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

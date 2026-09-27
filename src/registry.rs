//! Extension registry: the explicit, per-renderer equivalent of upstream's
//! global `ms.addSymbolPart`, `ms.addIconParts`, `ms.addIcons` and
//! `ms.addLabelOverrides`.

use crate::bbox::PartialBBox;
use crate::color::ColorSet;
use crate::compose::{BuiltinPart, SymbolPart};
use crate::ir::Node;
use crate::labels::LabelField;
use crate::metadata::Metadata;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A stage of the composition pipeline.
pub(crate) enum PartSlot {
    Builtin(BuiltinPart),
    Custom(Box<dyn SymbolPart>),
}

/// Context passed to [`IconExtension::icon_parts`] (upstream icon-part
/// function arguments).
pub struct IconPartContext<'a> {
    /// Metadata of the symbol being drawn.
    pub metadata: &'a Metadata,
    /// Resolved colours of the symbol.
    pub colors: &'a ColorSet,
    /// Whether MIL-STD-2525 rules apply.
    pub std2525: bool,
    /// Monochrome colour, empty for full colour.
    pub mono_color: &'a str,
    /// Alternate MEDAL icons requested.
    pub alternate_medal: bool,
}

/// Icons contributed for one SIDC table (a numeric symbol set, or the letter
/// SIDC table). Keys follow upstream: six-digit entity codes and two-digit
/// modifier codes for numeric SIDCs, generic SIDCs such as `S-G-UCI---` for
/// letter SIDCs.
#[derive(Debug, Clone, Default)]
pub struct IconTable {
    /// Main icons.
    pub icons: BTreeMap<String, Node>,
    /// Sector 1 modifiers (numeric SIDCs only).
    pub modifier1: BTreeMap<String, Node>,
    /// Sector 2 modifiers (numeric SIDCs only).
    pub modifier2: BTreeMap<String, Node>,
    /// Icon bounding boxes that differ from the default octagon.
    pub bbox: BTreeMap<String, PartialBBox>,
}

/// Read access to icon parts: extension parts first, then built-ins.
pub trait PartLookup {
    /// The named icon part, if defined.
    fn part(&self, name: &str) -> Option<Node>;
}

/// An icon extension (upstream `ms.addIcons` object).
///
/// All methods have empty defaults; implement the ones you need. Extensions
/// are consulted in registration order after the built-in tables, so later
/// definitions replace earlier ones, as in upstream.
pub trait IconExtension: Send + Sync {
    /// Adds or replaces named icon parts.
    fn icon_parts(&self, _ctx: &IconPartContext<'_>, _parts: &mut BTreeMap<String, Node>) {}

    /// Adds numeric-SIDC icons for `symbol_set` (two digits).
    fn number_icons(
        &self,
        _symbol_set: &str,
        _parts: &dyn PartLookup,
        _std2525: bool,
        _edition: Option<&str>,
        _out: &mut IconTable,
    ) {
    }

    /// Adds letter-SIDC icons.
    fn letter_icons(&self, _parts: &dyn PartLookup, _std2525: bool, _out: &mut IconTable) {}

    /// Adds label overrides for numeric SIDCs, keyed by entity code.
    fn number_labels(&self, _out: &mut BTreeMap<String, Vec<LabelField>>) {}

    /// Adds label overrides for letter SIDCs, keyed by generic SIDC.
    fn letter_labels(&self, _out: &mut BTreeMap<String, Vec<LabelField>>) {}
}

/// Pipeline and extensions of a renderer.
pub(crate) struct Registry {
    pub parts: Vec<PartSlot>,
    pub icons: Vec<Box<dyn IconExtension>>,
    pub number_labels: BTreeMap<String, Vec<LabelField>>,
    pub letter_labels: BTreeMap<String, Vec<LabelField>>,
}

impl Default for Registry {
    fn default() -> Self {
        Registry {
            parts: BuiltinPart::DEFAULT
                .iter()
                .map(|&p| PartSlot::Builtin(p))
                .collect(),
            icons: Vec::new(),
            number_labels: BTreeMap::new(),
            letter_labels: BTreeMap::new(),
        }
    }
}

impl Registry {
    pub(crate) fn add_icons(&mut self, ext: Box<dyn IconExtension>) {
        ext.number_labels(&mut self.number_labels);
        ext.letter_labels(&mut self.letter_labels);
        self.icons.push(ext);
    }
}

//! Extension registry: the explicit, per-renderer equivalent of upstream's
//! global `ms.addSymbolPart`, `ms.addIconParts`, `ms.addIcons` and
//! `ms.addLabelOverrides`.

use crate::bbox::PartialBBox;
use crate::color::ColorSet;
use crate::compat::JsMetadata;
use crate::compose::{BuiltinPart, SymbolPart};
use crate::domain::Metadata;
use crate::ir::Node;
use crate::labels::{self, LabelField};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// A stage of the composition pipeline.
pub(crate) enum PartSlot {
    Builtin(BuiltinPart),
    Custom(Box<dyn SymbolPart>),
}

/// The symbol an [`IconExtension`] is asked about (upstream icon-part and
/// icon function arguments).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct IconPartContext<'a> {
    /// Typed metadata of the symbol being drawn.
    pub metadata: &'a Metadata,
    /// Metadata in milsymbol.js's representation.
    pub js_metadata: &'a JsMetadata<'a>,
    /// Resolved colours of the symbol.
    pub colors: &'a ColorSet,
    /// Monochrome colour, empty for full colour.
    pub mono_color: &'a str,
    /// Alternate MEDAL icons requested.
    pub alternate_medal: bool,
}

/// Which icon an [`IconExtension`] is asked for. Keys follow upstream:
/// six-digit entity codes and two- or three-digit modifier codes for
/// numeric SIDCs, generic SIDCs such as `S-G-UCI---` for letter SIDCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IconKey<'k> {
    /// Main icon of a numeric SIDC.
    Entity {
        /// Two-digit symbol set.
        symbol_set: &'k str,
        /// Six-digit entity code.
        entity: &'k str,
    },
    /// Sector 1 modifier of a numeric SIDC.
    Modifier1 {
        /// Two-digit symbol set.
        symbol_set: &'k str,
        /// The two-digit field for the symbol set's own modifiers (`"07"`),
        /// or the complete three-digit code when a 30-digit SIDC selects
        /// another modifier set (`"107"`); see
        /// [`NumericSidc::modifier_codes`](crate::sidc::NumericSidc::modifier_codes).
        code: &'k str,
    },
    /// Sector 2 modifier of a numeric SIDC.
    Modifier2 {
        /// Two-digit symbol set.
        symbol_set: &'k str,
        /// The two-digit field for the symbol set's own modifiers (`"07"`),
        /// or the complete three-digit code when a 30-digit SIDC selects
        /// another modifier set (`"107"`); see
        /// [`NumericSidc::modifier_codes`](crate::sidc::NumericSidc::modifier_codes).
        code: &'k str,
    },
    /// Icon of a letter SIDC, by generic SIDC.
    Letter {
        /// Generic SIDC, e.g. `S-G-UCI---`.
        generic: &'k str,
    },
}

/// Read access to icon parts: extension parts first, then built-ins.
pub trait PartLookup {
    /// The named icon part, if defined.
    fn part(&self, name: &str) -> Option<Node>;
}

/// An icon extension (upstream `ms.addIconParts`, `ms.addIcons` and
/// `ms.addLabelOverrides`).
///
/// Extensions are queried by key while a symbol is drawn, so nothing is
/// built per symbol for keys they do not define. All methods have empty
/// defaults. Later registrations take precedence over earlier ones and over
/// the built-in tables, as in upstream.
/// Callbacks must return the same result for the same context and registry;
/// deterministic rendering and caching depend on this contract.
///
/// ```
/// use milsymbol::ir::{Node, Paint};
/// use milsymbol::{IconExtension, IconPartContext, PartLookup, Renderer};
///
/// /// Draws infantry as a circle, wherever the built-in part is used.
/// struct RoundInfantry;
///
/// impl IconExtension for RoundInfantry {
///     fn icon_part(&self, _: &IconPartContext<'_>, name: &str, _: &dyn PartLookup) -> Option<Node> {
///         (name == "GR.IC.FF.INFANTRY").then(|| {
///             let mut circle = Node::circle(100.0, 100.0, 25.0);
///             if let Some(style) = circle.style_mut() {
///                 style.fill = Some(Paint::None);
///                 style.stroke = Some(Paint::color("black"));
///             }
///             circle
///         })
///     }
/// }
///
/// let r = Renderer::default().with_icons(RoundInfantry);
/// let svg = r.symbol("10031000001211000000").render()?.to_svg();
/// assert!(svg.contains("<circle"));
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
pub trait IconExtension: Send + Sync {
    /// Adds or replaces the icon part `name`. `parts` resolves parts as
    /// defined before this extension (earlier extensions, then built-ins).
    fn icon_part(
        &self,
        _ctx: &IconPartContext<'_>,
        _name: &str,
        _parts: &dyn PartLookup,
    ) -> Option<Node> {
        None
    }

    /// Adds or replaces the icon for `key`. `parts` resolves parts with all
    /// extensions applied.
    fn icon(
        &self,
        _ctx: &IconPartContext<'_>,
        _key: IconKey<'_>,
        _parts: &dyn PartLookup,
    ) -> Option<Node> {
        None
    }

    /// Bounds of the icon for `key` when they differ from the octagon
    /// (entity and letter keys only).
    fn icon_bbox(&self, _ctx: &IconPartContext<'_>, _key: IconKey<'_>) -> Option<PartialBBox> {
        None
    }

    /// Adds label overrides for numeric control measures, keyed by the
    /// six-digit entity code. Other numeric symbol sets use the standard
    /// information-field layout, matching upstream. Called once when the
    /// extension is registered; omitted placement values use [`Label`](crate::labels::Label)
    /// defaults.
    fn number_labels(&self, _out: &mut BTreeMap<String, Vec<LabelField>>) {}

    /// Adds label overrides for letter SIDCs, keyed by generic SIDC. Called
    /// once when the extension is registered; omitted placement values use
    /// [`Label`](crate::labels::Label) defaults.
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
        for fields in self
            .number_labels
            .values_mut()
            .chain(self.letter_labels.values_mut())
        {
            labels::resolve_defaults(fields);
        }
        self.icons.push(ext);
    }
}

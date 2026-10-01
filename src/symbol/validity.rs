//! Symbol validity, with typed reasons.

use super::Symbol;
use crate::compat::UpstreamIssue;
use crate::ir::{Node, Num, Paint, Style};
use alloc::vec::Vec;

/// Why a symbol is not valid.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ValidityIssue {
    /// The standard identity is not recognised.
    UnknownAffiliation,
    /// The battle dimension or symbol set is not recognised.
    UnknownDimension,
    /// No icon exists for the function/entity or sector modifier codes.
    UnknownIcon,
    /// The echelon/mobility amplifier code is not recognised.
    UnknownAmplifier,
    /// An icon references a part that does not exist.
    MissingInstruction,
    /// The SIDC fails [`Sidc::parse`](crate::sidc::Sidc::parse). milsymbol.js
    /// renders such codes and may count them as valid.
    MalformedSidc,
}

/// Why a symbol is or is not valid.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Validity {
    /// Every reason the symbol is not valid; empty for a valid symbol.
    pub issues: Vec<ValidityIssue>,
}

impl Validity {
    /// Whether there are no issues.
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// The conditions both verdicts are built from.
struct Flags {
    affiliation: bool,
    dimension: bool,
    icon: bool,
    amplifier: bool,
    instruction: bool,
}

/// `icon_found` differs between the verdicts: upstream judges the drawn icon
/// (always found when icons are hidden), the SIDC verdict the SIDC's own.
fn flags(s: &Symbol, icon_found: bool) -> Flags {
    let md = &s.metadata;
    Flags {
        affiliation: md.affiliation == crate::metadata::Field::Undefined,
        dimension: md.dimension == crate::metadata::Field::Undefined && !md.control_measure(),
        icon: !icon_found,
        amplifier: md.mobility == crate::metadata::Field::Missing,
        instruction: crate::ir::contains_missing(&s.instructions),
    }
}

/// The issues of the SIDC verdict, in declaration order. Allocates only when
/// there is at least one.
pub(super) fn sidc_issues(s: &Symbol) -> Vec<ValidityIssue> {
    let f = flags(s, s.icon_known);
    [
        (f.affiliation, ValidityIssue::UnknownAffiliation),
        (f.dimension, ValidityIssue::UnknownDimension),
        (f.icon, ValidityIssue::UnknownIcon),
        (f.amplifier, ValidityIssue::UnknownAmplifier),
        (f.instruction, ValidityIssue::MissingInstruction),
        (
            crate::sidc::Sidc::parse(&s.sidc).is_err(),
            ValidityIssue::MalformedSidc,
        ),
    ]
    .into_iter()
    .filter_map(|(present, issue)| present.then_some(issue))
    .collect()
}

/// The issues of milsymbol.js `isValid()`, in declaration order.
pub(super) fn upstream_issues(s: &Symbol) -> Vec<UpstreamIssue> {
    let f = flags(s, s.valid_icon);
    [
        (f.affiliation, UpstreamIssue::UnknownAffiliation),
        (f.dimension, UpstreamIssue::UnknownDimension),
        (f.icon, UpstreamIssue::UnknownIcon),
        (f.amplifier, UpstreamIssue::UnknownAmplifier),
        (f.instruction, UpstreamIssue::MissingInstruction),
        (contains_null(&s.instructions), UpstreamIssue::NullInDrawing),
    ]
    .into_iter()
    .filter_map(|(present, issue)| present.then_some(issue))
    .collect()
}

/// Whether `JSON.stringify(nodes)` would contain `null` other than for a
/// missing instruction: a non-finite number or a string containing `null`.
fn contains_null(nodes: &[Node]) -> bool {
    nodes.iter().any(node_null)
}

fn num_null(n: &Num) -> bool {
    match n {
        Num::Number(v) => !v.is_finite(),
        Num::Text(t) => t.contains("null"),
        Num::Bool(_) => false,
    }
}

fn style_null(st: &Style) -> bool {
    let paint = |p: &Option<Paint>| matches!(p, Some(Paint::Color(c)) if c.contains("null"));
    let text = |t: &Option<crate::ir::Str>| t.as_deref().is_some_and(|t| t.contains("null"));
    paint(&st.fill)
        || paint(&st.stroke)
        || st.fill_opacity.as_ref().is_some_and(num_null)
        || st.stroke_width.as_ref().is_some_and(num_null)
        || text(&st.stroke_dasharray)
        || text(&st.line_cap)
        || text(&st.clip_path)
        || st.non_scaling_stroke.is_some_and(|v| !v.is_finite())
}

fn node_null(n: &Node) -> bool {
    let text = |t: &Option<crate::ir::Str>| t.as_deref().is_some_and(|t| t.contains("null"));
    let own = match n {
        Node::Missing => false,
        Node::Scalar(v) => num_null(v),
        Node::TrustedSvg(s) => s.contains("null"),
        Node::Group(v) => return contains_null(v),
        Node::Path(p) => p.d.source().contains("null"),
        Node::Circle(c) => num_null(&c.cx) || num_null(&c.cy) || num_null(&c.r),
        Node::Text(t) => {
            num_null(&t.x)
                || num_null(&t.y)
                || t.text.contains("null")
                || t.font_size.as_ref().is_some_and(num_null)
                || text(&t.font_family)
                || text(&t.font_weight)
                || text(&t.text_anchor)
                || text(&t.alignment_baseline)
        }
        Node::Translate(t) => num_null(&t.x) || num_null(&t.y),
        Node::Rotate(r) => num_null(&r.degree) || num_null(&r.x) || num_null(&r.y),
        Node::Scale(s) => num_null(&s.factor),
        Node::Clip(c) => c.d.source().contains("null") || text(&c.clip_id),
        Node::Bare(_) => false,
    };
    own || n.style().is_some_and(style_null) || n.children().is_some_and(contains_null)
}

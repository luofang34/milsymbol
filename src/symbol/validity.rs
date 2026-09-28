//! Symbol validity (upstream `isValid`), with typed reasons.

use super::Symbol;
use crate::ir::{Node, Num, Paint, Style};
use alloc::string::String;
use alloc::vec::Vec;

/// Why a symbol is not valid.
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
    /// A text or attribute contains `null`, or a coordinate is not finite.
    /// milsymbol.js counts this as invalid even when the SIDC is fine.
    NullInDrawing,
}

/// Detailed validity (upstream `isValid(true)`) plus typed issues.
#[derive(Debug, Clone, PartialEq)]
pub struct Validity {
    /// Affiliation the symbol was drawn with.
    pub affiliation: Option<String>,
    /// Dimension the symbol was drawn with.
    pub dimension: String,
    /// The battle dimension is unknown.
    pub dimension_unknown: bool,
    /// No draw instruction is missing and no value serializes as `null`.
    pub draw_instructions: bool,
    /// The icon was found.
    pub icon: bool,
    /// The mobility code was recognised.
    pub mobility: bool,
    /// Every reason the symbol is not valid; empty for a valid symbol.
    pub issues: Vec<ValidityIssue>,
}

/// The reasons `s` is not valid, in declaration order. Allocates only when
/// there is at least one.
pub(super) fn issues(s: &Symbol) -> Vec<ValidityIssue> {
    let md = &s.metadata;
    [
        (
            md.affiliation.as_deref() == Some("undefined"),
            ValidityIssue::UnknownAffiliation,
        ),
        (
            md.dimension == "undefined" && !md.control_measure(),
            ValidityIssue::UnknownDimension,
        ),
        (!s.valid_icon, ValidityIssue::UnknownIcon),
        (md.mobility.is_none(), ValidityIssue::UnknownAmplifier),
        (
            crate::ir::contains_missing(&s.instructions),
            ValidityIssue::MissingInstruction,
        ),
        (contains_null(&s.instructions), ValidityIssue::NullInDrawing),
    ]
    .into_iter()
    .filter_map(|(present, issue)| present.then_some(issue))
    .collect()
}

pub(super) fn of(s: &Symbol) -> Validity {
    let md = &s.metadata;
    let issues = issues(s);
    let drawing_ok = !issues.iter().any(|i| {
        matches!(
            i,
            ValidityIssue::MissingInstruction | ValidityIssue::NullInDrawing
        )
    });
    Validity {
        affiliation: md.affiliation.clone(),
        dimension: md.dimension.clone(),
        dimension_unknown: md.dimension_unknown,
        draw_instructions: drawing_ok,
        icon: s.valid_icon,
        mobility: md.mobility.is_some(),
        issues,
    }
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

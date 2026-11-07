//! The frame (upstream `basegeometry.js`) and the frame builder shared with
//! the stack part.

use super::{PartOutput, SymbolState};
use crate::bbox::BBox;
use crate::domain::Affiliation;
use crate::geometry::GeomShape;
use crate::ir::{CircleNode, Node, Num, Paint, PathData, PathNode, Style};
use crate::node_list::NodeAcc;
use alloc::string::String;
use alloc::vec::Vec;

fn affiliation_modifier(
    aff: Option<Affiliation>,
    friend: &'static str,
    hostile: &'static str,
    neutral: &'static str,
    unknown: &'static str,
) -> Option<&'static str> {
    match aff {
        Some(Affiliation::Friend) => Some(friend),
        Some(Affiliation::Hostile) => Some(hostile),
        Some(Affiliation::Neutral) => Some(neutral),
        Some(Affiliation::Unknown) => Some(unknown),
        _ => None,
    }
}

const SPACE: [&str; 4] = [
    "M 100,30 C 90,30 80,35 68.65625,50 l 62.6875,0 C 120,35 110,30 100,30",
    "M67,50 L100,20 133,50 z",
    "M45,50 l0,-20 110,0 0,20 z",
    "M 100 22.5 C 85 22.5 70 31.669211 66 50 L 134 50 C 130 31.669204 115 22.5 100 22.5 z",
];
const ACTIVITY: [&str; 4] = [
    "m 160,135 0,15 15,0 0,-15 z m -135,0 15,0 0,15 -15,0 z m 135,-85 0,15 15,0 0,-15 z m -135,0 15,0 0,15 -15,0 z",
    "M 100 28 L 89.40625 38.59375 L 100 49.21875 L 110.59375 38.59375 L 100 28 z M 38.6875 89.3125 L 28.0625 99.9375 L 38.6875 110.53125 L 49.28125 99.9375 L 38.6875 89.3125 z M 161.40625 89.40625 L 150.78125 100 L 161.40625 110.59375 L 172 100 L 161.40625 89.40625 z M 99.9375 150.71875 L 89.3125 161.3125 L 99.9375 171.9375 L 110.53125 161.3125 L 99.9375 150.71875",
    "m 140,140 15,0 0,15 -15,0 z m -80,0 0,15 -15,0 0,-15 z m 80,-80 0,-15 15,0 0,15 z m -80,0 -15,0 0,-15 15,0 z",
    "M 107.96875 31.46875 L 92.03125 31.71875 L 92.03125 46.4375 L 107.71875 46.4375 L 107.96875 31.46875 z M 47.03125 92.5 L 31.09375 92.75 L 31.09375 107.5 L 46.78125 107.5 L 47.03125 92.5 z M 168.4375 92.5 L 152.5 92.75 L 152.5 107.5 L 168.1875 107.5 L 168.4375 92.5 z M 107.96875 153.5625 L 92.03125 153.8125 L 92.03125 168.53125 L 107.71875 168.53125 L 107.96875 153.5625 z",
];
const CYBERSPACE: [&str; 4] = [
    "m 135,150 40,-40 0,40 z",
    "m 150,78 0,44 22,-22 z",
    "m 115,155 40,-40 0,40 z",
    "M 150 65.7 L 150 134 C 176 123 176 77.2 150 65.7 z",
];

/// Frame decoration for an affiliation, or [`Node::Missing`] when upstream
/// indexes its table with an unknown affiliation.
fn decoration(
    table: &[&'static str; 4],
    aff: Option<Affiliation>,
    frame_color: &Option<Paint>,
) -> Node {
    let [f, h, n, u] = *table;
    match affiliation_modifier(aff, f, h, n, u) {
        Some(d) => {
            let style = Style {
                stroke: Some(Paint::None),
                fill: frame_color.clone(),
                ..Style::default()
            };
            Node::Path(PathNode {
                d: PathData::new(d),
                style,
            })
        }
        None => Node::Missing,
    }
}

fn shape_node(shape: GeomShape, style: Style) -> Node {
    match shape {
        GeomShape::Path(d) => Node::Path(PathNode {
            d: PathData::new(d),
            style,
        }),
        GeomShape::Circle { cx, cy, r } => Node::Circle(CircleNode {
            cx: cx.into(),
            cy: cy.into(),
            r: r.into(),
            style,
        }),
    }
}

/// Appends frame instructions into `post` (and optional outline into `pre_out`),
/// returning `false` when upstream draws no frame. Always sets `bbox_out`.
pub(super) fn frame_into(
    s: &SymbolState<'_>,
    pre_out: &mut Option<Node>,
    post: &mut NodeAcc,
    bbox_out: &mut BBox,
) -> Result<bool, crate::RenderError> {
    let (md, st) = (s.metadata, &s.options.style);
    let Some(g) = md.geometry() else {
        *bbox_out = BBox::default();
        return Ok(false);
    };
    *bbox_out = g.bbox();
    if !md.frame && st.icon {
        return Ok(false);
    }
    let aff = s.aff();
    let frame_color = s.color_of(&s.colors.frame_color);
    let stroke_width = if st.size >= 10.0 {
        st.stroke_width
    } else {
        10.0
    };
    let fill = match &st.fill_color {
        None => s.color_of(&s.colors.fill_color),
        Some(c) => Some(Paint::Color(c.to_str())),
    };
    let mut geom_style = Style {
        fill,
        fill_opacity: Some(Num::Number(st.fill_opacity)),
        stroke: frame_color.clone(),
        stroke_width: Some(Num::Number(stroke_width)),
        ..Style::default()
    };
    if st.frame && st.outline_width > 0.0 {
        let outline = match g.shape {
            GeomShape::Path(d) if md.fill && st.mono_color.is_none() => {
                let mut o = PathNode {
                    d: PathData::new(String::from(d) + " Z"),
                    style: Style::default(),
                };
                o.style.stroke_width = Some(Num::Number(stroke_width));
                Node::Path(o)
            }
            shape => shape_node(shape, geom_style.clone()),
        };
        *pre_out = Some(s.outline_one(&outline)?);
    }
    if (st.mono_color.is_some() || !st.fill) && !md.notpresent.is_empty() {
        geom_style.stroke_dasharray = Some(md.notpresent.clone());
    }
    post.push(shape_node(g.shape, geom_style));
    if md.space {
        post.push(decoration(&SPACE, aff, &frame_color));
    }
    if md.activity {
        post.push(decoration(&ACTIVITY, aff, &frame_color));
    }
    if md.flags.cyberspace == Some(true) {
        post.push(decoration(&CYBERSPACE, aff, &frame_color));
    }
    if st.fill && st.frame && !md.notpresent.is_empty() {
        let dashed = Style {
            fill: Some(Paint::None),
            stroke: s.color_of(&s.colors.white),
            stroke_width: Some(Num::Number(st.stroke_width + 1.0)),
            stroke_dasharray: Some(md.notpresent.clone()),
            ..Style::default()
        };
        post.push(shape_node(g.shape, dashed));
    }
    Ok(true)
}

pub(super) fn render_direct(
    s: &SymbolState<'_>,
    acc: &mut NodeAcc,
    bbox: &mut BBox,
) -> Result<(), crate::RenderError> {
    let mut pre = None;
    let mut base = BBox::default();
    if frame_into(s, &mut pre, acc, &mut base)? {
        if let Some(o) = pre {
            acc.prepend_one(o);
        }
        bbox.merge(base.into());
    }
    Ok(())
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, crate::RenderError> {
    let mut pre = None;
    let mut post = NodeAcc::default();
    let mut bbox = BBox::default();
    let _ = frame_into(s, &mut pre, &mut post, &mut bbox)?;
    let pre_vec = pre.map_or_else(Vec::new, |o| alloc::vec![o]);
    Ok(PartOutput::new(pre_vec, post.into_vec(), bbox))
}

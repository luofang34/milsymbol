//! Placement of information fields around the frame.

use super::{SymbolState, TextStyle, fields};
use crate::bbox::BBox;
use crate::color::truthy;
use crate::compose::{s as lit, style_color_value};
use crate::domain::{Affiliation, Dimension};
use crate::ir::{Node, Num, Paint, PathData, PathNode, Str, Style};
use crate::js::{self, number_to_string as n};
use crate::labels::str_width;
use crate::options::field as f;
use alloc::string::String;
use alloc::vec::Vec;

const SPACE: f64 = 20.0;

/// Upstream's local `text()`: a centred label inside the frame.
fn centre_text(ts: &TextStyle, s: &str) -> Node {
    let size = match js::utf16_len(s) {
        3 => 39.0,
        len if len >= 4 => 33.0,
        _ => 45.0,
    };
    let mut t = crate::ir::TextNode::new(100.0, 103.0, Str::Owned(String::from(s)));
    t.style.stroke = Some(Paint::None);
    t.text_anchor = lit("middle");
    t.alignment_baseline = lit("middle");
    t.font_size = Some(Num::Number(size));
    t.font_weight = lit("bold");
    t.font_family = Some(ts.family.clone());
    t.style.fill = ts.color.clone();
    Node::Text(t)
}

fn js_max(values: &[f64]) -> f64 {
    values.iter().copied().fold(f64::NEG_INFINITY, js::max)
}

/// Extra right-hand offsets: (flag space for R4/R5, stack offset).
fn offsets(s: &SymbolState<'_>) -> (f64, f64) {
    let (md, opts) = (s.metadata, s.options);
    let mut flag = if opts.country_flag.as_deref().is_some_and(|c| !c.is_empty()) {
        70.0
    } else {
        0.0
    };
    if opts.full_frame_flag == Some(true)
        && md.affiliation.known() == Some(Affiliation::Friend)
        && md.dimension.known() == Some(Dimension::Ground)
    {
        flag = 0.0;
    }
    flag += if opts.signature.as_deref() == Some("!") {
        30.0
    } else {
        0.0
    };
    let stack = match opts.stack {
        Some(v) if v != 0.0 && !v.is_nan() => v * 15.0,
        _ => 0.0,
    };
    (flag, stack)
}

/// Texts centred on the frame: special headquarters, quantity, HQ element.
fn centred_fields(
    s: &SymbolState<'_>,
    ts: &TextStyle,
    bbox: &BBox,
    post: &mut Vec<Node>,
    gbbox: &mut BBox,
) {
    let (md, opts) = (s.metadata, s.options);
    let special_hq = opts.text_named(f::SPECIAL_HEADQUARTERS);
    let quantity = opts.text_named(f::QUANTITY);
    if !special_hq.is_empty() {
        post.push(centre_text(ts, special_hq));
    }
    if !quantity.is_empty() && !md.dismounted() {
        post.push(ts.text(quantity, 100.0, bbox.y1 - 10.0, "middle"));
        gbbox.y1 = bbox.y1 - 10.0 - ts.size;
    }
    let hq_element = opts.text_named(f::HEADQUARTERS_ELEMENT);
    if !hq_element.is_empty() {
        let mut t = ts.text(hq_element, 100.0, bbox.y2 + 35.0, "middle");
        if let Node::Text(t) = &mut t {
            t.font_size = Some(Num::Number(35.0));
            t.font_weight = lit("bold");
        }
        post.push(t);
        gbbox.y2 = bbox.y2 + 35.0;
    }
}

/// Grows `gbbox` to fit the left/right fields.
fn extent(
    s: &SymbolState<'_>,
    g: &fields::Strings,
    bbox: &BBox,
    fs: f64,
    (flag, stack): (f64, f64),
    gbbox: &mut BBox,
) {
    let opts = s.options;
    let centred = |v: &str| {
        if v.is_empty() {
            0.0
        } else {
            (str_width(v, fs, SPACE) - bbox.width()) / 2.0
        }
    };
    let (hq_w, q_w) = (
        centred(opts.text_named(f::SPECIAL_HEADQUARTERS)),
        centred(opts.text_named(f::QUANTITY)),
    );
    let [l1, l2, l3, l4, l5] = &g.l;
    let [r1, r2, r3, r4, r5] = &g.r;
    let w = |v: &str, space: f64| str_width(v, fs, space);
    gbbox.x1 = bbox.x1
        - js_max(&[
            hq_w,
            q_w,
            w(l1, SPACE),
            w(l2, SPACE),
            w(l3, SPACE),
            w(l4, SPACE),
            w(l5, SPACE),
        ]);
    let (rs, rsf) = (SPACE + stack, SPACE + stack + flag * 1.5);
    gbbox.x2 = bbox.x2
        + js_max(&[
            hq_w,
            q_w,
            w(r1, rs),
            w(r2, rs),
            w(r3, rs),
            w(r4, rsf),
            w(r5, rsf),
        ]);
    let any = |a: &str, b: &str| !a.is_empty() || !b.is_empty();
    if any(l1, r1) {
        gbbox.y1 = js::min(gbbox.y1, 100.0 - 2.5 * fs);
    }
    if any(l2, r2) {
        gbbox.y1 = js::min(gbbox.y1, 100.0 - 1.5 * fs);
    }
    if any(l4, r4) {
        gbbox.y2 = js::max(gbbox.y2, 100.0 + 1.7 * fs);
    }
    if any(l5, r5) {
        gbbox.y2 = js::max(gbbox.y2, 100.0 + 2.7 * fs);
    }
}

pub(super) fn draw(s: &SymbolState<'_>, ts: &TextStyle, post: &mut Vec<Node>, gbbox: &mut BBox) {
    let md = s.metadata;
    let bbox = md.geometry_bbox();
    let fs = ts.size;
    let (flag, stack) = offsets(s);
    centred_fields(s, ts, &bbox, post, gbbox);
    let g = fields::compute(s);
    let quantity = s.options.text_named(f::QUANTITY);
    if md.dismounted() && !quantity.is_empty() {
        post.push(ts.text(quantity, 100.0, bbox.y2 + fs, "middle"));
        gbbox.y2 = bbox.y2 + fs;
    }
    extent(s, &g, &bbox, fs, (flag, stack), gbbox);
    if s.options.style.info_background.is_some() {
        backgrounds(s, &g, &bbox, fs, post, gbbox);
    }
    let rows = [-1.5, -0.5, 0.5, 1.5, 2.5];
    for (text, dy) in g.l.iter().zip(rows) {
        if !text.is_empty() {
            post.push(ts.text(text, bbox.x1 - SPACE, 100.0 + dy * fs, "end"));
        }
    }
    for (i, (text, dy)) in g.r.iter().zip(rows).enumerate() {
        if !text.is_empty() {
            let x = if i >= 3 {
                flag + bbox.x2 + SPACE + stack
            } else {
                bbox.x2 + SPACE + stack
            };
            post.push(ts.text(text, x, 100.0 + dy * fs, "start"));
        }
    }
}

/// Vertical extent of row `i` of the background boxes.
fn row_extent(i: usize, fs: f64) -> (f64, f64) {
    let top = [2.5, 1.5, 0.5, -0.5, -1.5];
    let bottom = [1.5, 0.5, -0.5, -1.5, -2.5];
    let t = top.get(i).copied().unwrap_or(0.0);
    let b = bottom.get(i).copied().unwrap_or(0.0);
    (100.0 - t * fs, 100.0 - b * fs + SPACE / 2.0)
}

fn backgrounds(
    s: &SymbolState<'_>,
    g: &fields::Strings,
    bbox: &BBox,
    fs: f64,
    post: &mut Vec<Node>,
    gbbox: &mut BBox,
) {
    let fill = style_color_value(s.options.style.info_background.as_ref(), s.aff());
    let stroke = if truthy(&fill) {
        fill.clone()
    } else {
        Some(Paint::None)
    };
    let style = || Style {
        fill: fill.clone(),
        stroke: stroke.clone(),
        ..Style::default()
    };
    let (mut lx1, mut lx2, mut ly1, mut ly2) = (100.0, None, 1000.0, 0.0);
    for (i, text) in g.l.iter().enumerate() {
        if !text.is_empty() {
            let (t, b) = row_extent(i, fs);
            lx1 = js::min(lx1, bbox.x1 - str_width(text, fs, SPACE));
            lx2 = Some(bbox.x1 - SPACE / 2.0);
            ly1 = js::min(ly1, t);
            ly2 = js::max(ly2, b);
        }
    }
    if let Some(x2) = lx2 {
        gbbox.x1 -= fs / 2.0;
        let d = alloc::format!(
            "M {},{} {},{} {},{} {},{} {},{}z",
            n(lx1 - fs / 2.0),
            n(ly1 + fs / 2.0),
            n(lx1),
            n(ly1),
            n(x2),
            n(ly1),
            n(x2),
            n(ly2),
            n(lx1 - fs / 2.0),
            n(ly2)
        );
        post.push(Node::Path(PathNode {
            d: PathData::new(d),
            style: style(),
        }));
    }
    let (mut rx1, mut rx2, mut ry1, mut ry2) = (None, 100.0, 1000.0, 0.0);
    for (i, text) in g.r.iter().enumerate() {
        if !text.is_empty() {
            let (t, b) = row_extent(i, fs);
            rx1 = Some(bbox.x2 + SPACE / 2.0);
            rx2 = js::max(rx2, bbox.x2 + str_width(text, fs, SPACE));
            ry1 = js::min(ry1, t);
            ry2 = js::max(ry2, b);
        }
    }
    if let Some(x1) = rx1 {
        gbbox.x2 += fs / 2.0;
        let d = alloc::format!(
            "M {},{} {},{} {},{} {},{} {},{}z",
            n(x1),
            n(ry1),
            n(rx2 + fs / 2.0),
            n(ry1),
            n(rx2 + fs / 2.0),
            n(ry2 - fs / 2.0),
            n(rx2),
            n(ry2),
            n(x1),
            n(ry2)
        );
        post.push(Node::Path(PathNode {
            d: PathData::new(d),
            style: style(),
        }));
    }
}

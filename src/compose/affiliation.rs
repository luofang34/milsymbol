//! Exercise/simulation letters and the unknown-dimension question mark
//! (upstream `affliationdimension.js`).

use super::{PartOutput, SymbolState, s as lit};
use crate::bbox::{BBox, PartialBBox};
use crate::color::truthy;
use crate::domain::{Affiliation, Context, Dimension};
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, Str};
use alloc::vec::Vec;

fn letter(
    s: &SymbolState<'_>,
    text: &'static str,
    x: f64,
    y: f64,
    size: f64,
    anchor: &'static str,
    color: &Option<Paint>,
) -> Node {
    let mut t = crate::ir::TextNode::new(x, y, Str::Borrowed(text));
    t.style.fill = color.clone();
    t.font_family = Some(s.options.style.font_family.clone());
    t.font_size = Some(Num::Number(size));
    t.font_weight = lit("bold");
    t.text_anchor = lit(anchor);
    Node::Text(t)
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let md = s.metadata;
    let base = md.geometry_bbox();
    let mut bbox = PartialBBox::from(base);
    let frame_color = s.color_of(&s.colors.frame_color);
    let has_color = truthy(&frame_color);
    let mut post = Vec::new();
    if md.dimension_unknown && has_color {
        post.push(letter(s, "?", 100.0, 127.0, 80.0, "middle", &frame_color));
    }
    if md.geometry().is_some() && has_color {
        let aff = md.affiliation.known();
        let spacing = if aff == Some(Affiliation::Unknown)
            || (aff == Some(Affiliation::Hostile)
                && md.dimension.known() != Some(Dimension::Subsurface))
        {
            -10.0
        } else {
            10.0
        };
        let x = base.x2 + spacing;
        let mut side = |text: &'static str, y: f64| {
            let mut n = letter(s, text, x, y, 35.0, "start", &frame_color);
            if let Node::Text(t) = &mut n {
                t.alignment_baseline = lit("middle");
            }
            post.push(n);
        };
        match md.context {
            Some(Context::Exercise) => {
                if !(md.joker || md.faker) {
                    side("X", 50.0);
                }
                if md.joker {
                    side("J", 40.0);
                }
                if md.faker {
                    side("K", 40.0);
                }
                bbox = PartialBBox {
                    x2: Some(base.x2 + spacing + 22.0),
                    y1: Some(40.0 - 25.0),
                    ..PartialBBox::default()
                };
            }
            Some(Context::Simulation) => {
                side("S", 40.0);
                let full = BBox {
                    x2: base.x2 + spacing + 22.0,
                    y1: 40.0 - 25.0,
                    ..BBox::default()
                };
                bbox = full.into();
            }
            _ => {}
        }
    }
    let mut pre = Vec::new();
    if s.options.style.outline_width > 0.0 {
        pre.push(s.outline(&post)?);
    }
    Ok(PartOutput::new(pre, post, bbox))
}

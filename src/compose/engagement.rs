//! Engagement bar (upstream `engagmentbar.js`).

use super::{PartOutput, SymbolState, s as lit};
use crate::bbox::PartialBBox;
use crate::color::truthy;
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, PathData, PathNode, Str, Style};
use crate::js::{self, number_to_string as n};
use crate::options::field;
use alloc::string::String;
use alloc::vec::Vec;

/// `a || b` for colour values.
pub(super) fn or_color(a: Option<Paint>, b: Option<Paint>) -> Option<Paint> {
    if truthy(&a) { a } else { b }
}

fn bar_text(s: &SymbolState<'_>, bar: &str, y: f64) -> Node {
    let font_color = or_color(
        s.color_of(&s.colors.icon_color),
        s.colors.icon_color.get("Friend"),
    );
    let mut text = crate::ir::TextNode::new(100.0, y, Str::Owned(String::from(bar)));
    text.text_anchor = lit("middle");
    text.font_size = Some(Num::Number(22.0));
    text.font_family = Some(s.options.style.font_family.clone());
    text.font_weight = lit("bold");
    text.style.fill = font_color;
    text.style.stroke = Some(Paint::None);
    Node::Text(text)
}

/// Bar fill: the engagement type colour, else the frame fill; `false` when
/// the symbol is unfilled.
fn bar_fill(s: &SymbolState<'_>, filled: bool) -> Option<Paint> {
    if !filled {
        return Some(Paint::None);
    }
    let named = match s
        .options
        .text(field::ENGAGEMENT_TYPE)
        .to_uppercase()
        .as_str()
    {
        "TARGET" => Some(Paint::color("rgb(255, 0, 0)")),
        "NON-TARGET" => Some(Paint::color("rgb(255, 255, 255)")),
        "EXPIRED" => Some(Paint::color("rgb(255, 120, 0)")),
        _ => None,
    };
    or_color(named, s.color_of(&s.colors.fill_color))
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let (md, st) = (s.metadata, &s.options.style);
    let bbox = s.bbox;
    let (mut x1, mut x2, mut y1, y2) = (bbox.x1, bbox.x2, bbox.y1, bbox.y2);
    let bar = s.options.text(field::ENGAGEMENT_BAR);
    let mut pre = Vec::new();
    let mut post = Vec::new();
    if !bar.is_empty() {
        y1 -= 6.0;
        post.push(bar_text(s, bar, bbox.y1 - 11.0));
        let filled = md.fill && st.mono_color.is_empty();
        let color = bar_fill(s, filled);
        let width = js::max(bbox.width(), js::utf16_len(bar) as f64 * 16.0);
        x1 = js::min(x1, 100.0 - width / 2.0);
        x2 = js::max(x2, 100.0 + width / 2.0);
        let w = n(width);
        let d = String::from("M")
            + &n(100.0 - width / 2.0)
            + ","
            + &n(y1)
            + " l"
            + &w
            + ",0 0,-25 -"
            + &w
            + ",0 z";
        let style = Style {
            stroke_width: Some(Num::Number(st.stroke_width)),
            fill: color,
            stroke: s.color_of(&s.colors.frame_color),
            ..Style::default()
        };
        post.insert(
            0,
            Node::Path(PathNode {
                d: PathData::new(d),
                style,
            }),
        );
        y1 -= 25.0;
        if st.outline_width > 0.0 {
            let outline = match (filled, post.first()) {
                (true, Some(first)) => s.outline_one(first)?,
                _ => s.outline(&post)?,
            };
            pre.push(outline);
        }
    }
    Ok(PartOutput::new(
        pre,
        post,
        PartialBBox {
            x1: Some(x1),
            x2: Some(x2),
            y1: Some(y1),
            y2: Some(y2),
        },
    ))
}

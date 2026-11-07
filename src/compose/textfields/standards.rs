//! Variable values in the standard's meteorological point drawings.

use super::{PartOutput, SymbolState, TextStyle};
use crate::catalog::standards::{CodeKind, entry};
use crate::compose::icon::standards::GraphicInput;
use crate::generated::standards::GRAPHICS;
use crate::ir::{Node, Paint};
use crate::{BBox, RenderError, RenderMode};
use alloc::vec::Vec;

pub(super) fn draw(s: &SymbolState<'_>) -> Result<Option<PartOutput>, RenderError> {
    if s.config.mode != RenderMode::Standards {
        return Ok(None);
    }
    let Some(version) = s
        .sidc
        .get(..2)
        .and_then(|v| v.parse::<u8>().ok())
        .filter(|v| matches!(v, 15 | 16))
    else {
        return Ok(None);
    };
    let Some(ss) = s.sidc.get(4..6) else {
        return Ok(None);
    };
    let Some(code) = s.sidc.get(10..16) else {
        return Ok(None);
    };
    let Some(input) = entry(version, ss, CodeKind::Entity, code)
        .and_then(|e| e.graphic)
        .and_then(|i| GRAPHICS.get(i))
        .and_then(|g| g.input.as_ref())
    else {
        return Ok(None);
    };
    let value = s.options.text_named(input.field);
    let mut post = Vec::new();
    let mut bbox = BBox::default();
    if s.options.style.info_fields && !value.is_empty() {
        let color = match &s.options.style.mono_color {
            Some(color) => Some(Paint::Color(color.to_str())),
            None if input.color == "icon" => s.color_of(&s.colors.icon_color),
            None => Some(Paint::color(input.color)),
        };
        let style = TextStyle {
            color,
            family: s.options.style.font_family.clone(),
            size: input.size,
        };
        values(input, &style, value, &mut post, &mut bbox);
    }
    let mut pre = Vec::new();
    if s.options.style.outline_width > 0.0 && !post.is_empty() {
        pre.push(s.outline(&post)?);
    }
    Ok(Some(PartOutput::new(pre, post, bbox)))
}

fn values(
    input: &GraphicInput,
    style: &TextStyle,
    value: &str,
    post: &mut Vec<Node>,
    bbox: &mut BBox,
) {
    let mut node = style.text(value, input.x, input.y, "middle");
    if input.bold {
        if let Node::Text(t) = &mut node {
            t.font_weight = Some("bold".into());
        }
    }
    let mut width = crate::labels::str_width(value, style.size, 0.0);
    let mut bottom = input.y;
    if let Some((whole, fraction)) = value.split_once('.').filter(|_| input.subscript_decimal) {
        let small = TextStyle {
            size: style.size * 0.65,
            color: style.color.clone(),
            family: style.family.clone(),
        };
        let main_width = crate::labels::str_width(whole, style.size, 0.0);
        width = main_width + crate::labels::str_width(fraction, small.size, 0.0);
        let left = input.x - width / 2.0;
        node = style.text(whole, left, input.y, "start");
        bottom += style.size * 0.25;
        post.push(small.text(fraction, left + main_width, bottom, "start"));
    }
    post.push(node);
    bbox.merge(
        BBox {
            x1: input.x - width / 2.0,
            x2: input.x + width / 2.0,
            y1: input.y - style.size,
            y2: bottom,
        }
        .into(),
    );
}

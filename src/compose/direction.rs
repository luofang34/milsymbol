//! Direction of movement indicator and speed leader (upstream
//! `directionarrow.js`).

use super::engagement::or_color;
use super::{PartOutput, SymbolState};
use crate::bbox::BBox;
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, PathData, PathNode, RotateNode, Style};
use crate::js::{self, number_to_string as n};
use alloc::string::String;
use alloc::vec::Vec;
use core::f64::consts::PI;

fn line(d: String, color: &Option<Paint>, stroke_width: f64) -> Node {
    let style = Style {
        fill: color.clone(),
        stroke: color.clone(),
        stroke_width: Some(Num::Number(stroke_width)),
        ..Style::default()
    };
    Node::Path(PathNode {
        d: PathData::new(d),
        style,
    })
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let (md, st, opts) = (s.metadata, &s.options.style, s.options);
    let bbox = if md.geometry().is_none() {
        s.bbox
    } else {
        md.geometry_bbox()
    };
    let mut gbbox = BBox::default();
    let color = or_color(
        s.color_of(&s.colors.icon_color),
        s.colors.icon_color.get("Friend"),
    );
    let mut pre = Vec::new();
    let mut post = Vec::new();
    let Some(direction) = opts.direction.filter(|_| st.info_fields) else {
        return Ok(PartOutput::new(pre, post, gbbox));
    };
    let sw = st.stroke_width;
    let arrow = if opts.speed_leader == 0.0 {
        let len = 95.0;
        let head = line(
            String::from("M100,100 l0,-") + &n(len - 20.0) + " -5,3 5,-15 5,15 -5,-3",
            &color,
            sw,
        );
        let rotate = Node::Rotate(RotateNode {
            degree: Num::Number(direction),
            x: Num::Number(100.0),
            y: Num::Number(100.0),
            draw: alloc::vec![head],
            style: Style::default(),
        });
        let mut arrow = alloc::vec![rotate];
        let turn = (direction / 360.0) * PI * 2.0;
        let (c, sn) = (libm::cos(turn), libm::sin(turn));
        gbbox.y1 = js::min(100.0 - c * len, 100.0);
        gbbox.y2 = js::max(100.0 - c * len, 100.0);
        gbbox.x1 = js::min(100.0 + sn * len, 100.0);
        gbbox.x2 = js::max(100.0 + sn * len, 100.0);
        if md.base_dimension == "Ground" || md.base_dimension.is_empty() {
            if !md.headquarters {
                let stem = line(
                    String::from("M 100,") + &n(bbox.y2) + "l0," + &n(100.0),
                    &color,
                    sw,
                );
                arrow = alloc::vec![Node::translate(0.0, bbox.y2, arrow), stem];
            } else {
                let hq = if st.hq_staff_length != 0.0 && !st.hq_staff_length.is_nan() {
                    st.hq_staff_length
                } else {
                    s.config.hq_staff_length
                };
                arrow = alloc::vec![Node::translate(
                    bbox.x1 - 100.0,
                    bbox.y2 - (100.0 - hq),
                    arrow
                )];
                gbbox.x1 += bbox.x1 - 100.0;
                gbbox.x2 += bbox.x1 - 100.0;
            }
        }
        gbbox.y2 += bbox.y2 + sw;
        let arrow = Node::Group(arrow);
        post.push(arrow.clone());
        arrow
    } else {
        let length = opts.speed_leader * (100.0 / st.size);
        let rad = (direction * PI) / 180.0;
        let y = -length * libm::cos(rad);
        let x = length * libm::sin(rad);
        gbbox.x1 = js::min(100.0, 100.0 + x);
        gbbox.x2 = js::max(100.0, 100.0 + x);
        gbbox.y1 = js::min(100.0, 100.0 + y);
        gbbox.y2 = js::max(100.0, 100.0 + y);
        let arrow = line(
            String::from("M 100,100  l") + &n(x) + "," + &n(y),
            &color,
            sw,
        );
        pre.push(arrow.clone());
        arrow
    };
    if st.outline_width > 0.0 {
        let outline = match &arrow {
            Node::Group(list) => s.outline(list)?,
            other => s.outline_one(other)?,
        };
        pre.insert(0, outline);
    }
    Ok(PartOutput::new(pre, post, gbbox))
}

//! Direction of movement indicator and speed leader (upstream
//! `directionarrow.js`).

use super::engagement::or_color;
use super::{PartOutput, SymbolState};
use crate::bbox::BBox;
use crate::domain::Dimension;
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

/// Direction of movement arrow (speed leader length 0).
fn movement(
    s: &SymbolState<'_>,
    bbox: BBox,
    direction: f64,
    color: &Option<Paint>,
    gbbox: &mut BBox,
) -> Node {
    let (md, st) = (s.metadata, &s.options.style);
    let (sw, len) = (st.stroke_width, 95.0);
    let head = line(
        String::from("M100,100 l0,-") + &n(len - 20.0) + " -5,3 5,-15 5,15 -5,-3",
        color,
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
    let platform = s.config.reference_platform;
    let (c, sn) = (js::cos(turn, platform), js::sin(turn, platform));
    gbbox.y1 = js::min(100.0 - c * len, 100.0);
    gbbox.y2 = js::max(100.0 - c * len, 100.0);
    gbbox.x1 = js::min(100.0 + sn * len, 100.0);
    gbbox.x2 = js::max(100.0 + sn * len, 100.0);
    if md.base_dimension.known() == Some(Dimension::Ground)
        || md.base_dimension == crate::metadata::Field::Empty
    {
        if !md.headquarters {
            let stem = line(
                String::from("M 100,") + &n(bbox.y2) + "l0," + &n(100.0),
                color,
                sw,
            );
            arrow = alloc::vec![Node::translate(0.0, bbox.y2, arrow), stem];
        } else {
            let hq = st
                .hq_staff_length_override()
                .unwrap_or(s.config.hq_staff_length);
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
    Node::Group(arrow)
}

/// Speed leader: a line whose length is in pixels, independent of size.
fn speed_leader(
    s: &SymbolState<'_>,
    direction: f64,
    color: &Option<Paint>,
    gbbox: &mut BBox,
) -> Node {
    let st = &s.options.style;
    let length = s.options.speed_leader_px() * (100.0 / st.size);
    let rad = (direction * PI) / 180.0;
    let platform = s.config.reference_platform;
    let y = -length * js::cos(rad, platform);
    let x = length * js::sin(rad, platform);
    gbbox.x1 = js::min(100.0, 100.0 + x);
    gbbox.x2 = js::max(100.0, 100.0 + x);
    gbbox.y1 = js::min(100.0, 100.0 + y);
    gbbox.y2 = js::max(100.0, 100.0 + y);
    line(
        String::from("M 100,100  l") + &n(x) + "," + &n(y),
        color,
        st.stroke_width,
    )
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
    let arrow = if opts.speed_leader_px() == 0.0 {
        movement(s, bbox, direction, &color, &mut gbbox)
    } else {
        speed_leader(s, direction, &color, &mut gbbox)
    };
    if st.outline_width > 0.0 {
        let outline = match &arrow {
            Node::Group(list) => s.outline(list)?,
            other => s.outline_one(other)?,
        };
        pre.push(outline);
    }
    if opts.speed_leader_px() == 0.0 {
        post.push(arrow);
    } else {
        pre.push(arrow);
    }
    Ok(PartOutput::new(pre, post, gbbox))
}

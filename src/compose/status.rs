//! Operational condition (upstream `statusmodifier.js`).

use super::{PartOutput, SymbolState};
use crate::bbox::PartialBBox;
use crate::domain::Status;
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, PathData, PathNode, Style};
use crate::js::number_to_string as n;
use alloc::string::String;
use alloc::vec::Vec;

fn condition_color(condition: Option<Status>) -> Option<Paint> {
    Some(Paint::color(match condition {
        Some(Status::FullyCapable) => "rgb(0,255,0)",
        Some(Status::Damaged) => "rgb(255,255,0)",
        Some(Status::Destroyed) => "rgb(255,0,0)",
        Some(Status::FullToCapacity) => "rgb(0, 180, 240)",
        _ => return None,
    }))
}

fn slash(d: &'static str, s: &SymbolState<'_>) -> Node {
    let style = Style {
        stroke_width: Some(Num::Number(s.options.style.stroke_width * 2.0)),
        stroke: s.color_of(&s.colors.frame_color),
        ..Style::default()
    };
    Node::Path(PathNode {
        d: PathData::new(d),
        style,
    })
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let (md, st) = (s.metadata, &s.options.style);
    let bbox = md.geometry_bbox();
    let (mut y1, mut y2) = (bbox.y1, bbox.y2);
    let mut pre = Vec::new();
    let mut post = Vec::new();
    if md.condition.is_some() {
        if md.fill && st.mono_color.is_none() && !st.simple_status_modifier {
            if !s
                .options
                .text_named(crate::options::field::HEADQUARTERS_ELEMENT)
                .is_empty()
            {
                y2 += 35.0;
            }
            y2 += if md.mobility.known().is_some() {
                25.0
            } else {
                5.0
            };
            let w = n(bbox.width());
            let d = String::from("M")
                + &n(bbox.x1)
                + ","
                + &n(y2)
                + " l"
                + &w
                + ",0 0,25 -"
                + &w
                + ",0 z";
            let style = Style {
                stroke_width: Some(Num::Number(st.stroke_width)),
                fill: condition_color(md.condition),
                stroke: s.color_of(&s.colors.frame_color),
                ..Style::default()
            };
            post.push(Node::Path(PathNode {
                d: PathData::new(d),
                style,
            }));
            y2 += 25.0;
        } else {
            if md.condition == Some(Status::Damaged) || md.condition == Some(Status::Destroyed) {
                post.push(slash("M150,20 L50,180", s));
                y1 = 20.0;
                y2 = 180.0;
            }
            if md.condition == Some(Status::Destroyed) {
                post.push(slash("M50,20 L150,180", s));
            }
        }
        if st.outline_width > 0.0 {
            pre.push(s.outline(&post)?);
        }
    }
    Ok(PartOutput::new(
        pre,
        post,
        PartialBBox {
            y1: Some(y1),
            y2: Some(y2),
            ..PartialBBox::default()
        },
    ))
}

//! Frame modifiers (upstream `modifier.js`): headquarters staff, task force,
//! installation, feint/dummy, echelon, mobility and leadership.

use super::{PartOutput, SymbolState};
use crate::bbox::{BBox, PartialBBox};
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, PathData, PathNode, Style};
use crate::js::{self, number_to_string as n};
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

mod amplifiers;

/// Output under construction; upstream's `drawArray1`/`drawArray2`/`gbbox`.
pub(super) struct Acc {
    pub pre: Vec<Node>,
    pub post: Vec<Node>,
    pub gbbox: BBox,
    /// Modifier colour (`style.frameColor[aff]` or `colors.iconColor[aff]`).
    pub color: Option<Paint>,
    pub stroke_width: f64,
}

impl Acc {
    /// Upstream's final pass: `fill = false`, `stroke = color`,
    /// `strokewidth = style.strokeWidth` where absent.
    pub fn defaults(&self, extra: Style) -> Style {
        Style {
            fill: Some(extra.fill.unwrap_or(Paint::None)),
            stroke: extra.stroke.or_else(|| self.color.clone()),
            stroke_width: Some(extra.stroke_width.unwrap_or(Num::Number(self.stroke_width))),
            ..extra
        }
    }

    /// Outlines a raw leaf geometry (before defaults, as upstream does) and
    /// pushes it with default attributes applied.
    pub fn push_leaf(
        &mut self,
        s: &SymbolState<'_>,
        d: String,
        extra: Style,
    ) -> Result<(), RenderError> {
        let raw = Node::Path(PathNode {
            d: PathData::new(d),
            style: extra,
        });
        if s.options.style.outline_width > 0.0 {
            self.pre.push(s.outline_one(&raw)?);
        }
        let mut geom = raw;
        if let Some(st) = geom.style_mut() {
            *st = self.defaults(core::mem::take(st));
        }
        self.post.push(geom);
        Ok(())
    }

    /// Pushes a translated group and its outline.
    pub fn push_group(
        &mut self,
        s: &SymbolState<'_>,
        x: f64,
        y: f64,
        draw: Vec<Node>,
    ) -> Result<(), RenderError> {
        let group = Node::translate(x, y, draw);
        if s.options.style.outline_width > 0.0 {
            let mut outline = s.outline_one(&group)?;
            if let Some(st) = outline.style_mut() {
                *st = self.defaults(core::mem::take(st));
            }
            self.pre.push(outline);
        }
        let mut group = group;
        if let Some(st) = group.style_mut() {
            *st = self.defaults(Style::default());
        }
        self.post.push(group);
        Ok(())
    }
}

fn modifier_color(s: &SymbolState<'_>) -> Option<Paint> {
    let fc = &s.options.style.frame_color;
    if fc.is_set() {
        // A string frameColor is indexed by affiliation like an object and
        // yields `undefined` upstream.
        fc.as_mode().and_then(|m| m.get(s.aff()))
    } else {
        s.color_of(&s.colors.icon_color)
    }
}

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, RenderError> {
    let (md, st) = (s.metadata, &s.options.style);
    let mut bbox = md.geometry_bbox();
    let mut acc = Acc {
        pre: Vec::new(),
        post: Vec::new(),
        gbbox: BBox::default(),
        color: modifier_color(s),
        stroke_width: st.stroke_width,
    };
    let dim_aff = alloc::format!(
        "{}{}",
        md.dimension,
        md.affiliation.as_deref().unwrap_or("undefined")
    );
    let hq_len = if st.hq_staff_length != 0.0 && !st.hq_staff_length.is_nan() {
        st.hq_staff_length
    } else {
        s.config.hq_staff_length
    };
    if md.headquarters && hq_len > 0.0 {
        let full = [
            "AirFriend",
            "AirNeutral",
            "GroundFriend",
            "GroundNeutral",
            "SeaNeutral",
            "SubsurfaceNeutral",
        ];
        let y = if full.contains(&dim_aff.as_str()) {
            bbox.y2
        } else {
            100.0
        };
        acc.push_leaf(
            s,
            alloc::format!(
                "M{},{} L{},{}",
                n(bbox.x1),
                n(y),
                n(bbox.x1),
                n(bbox.y2 + hq_len)
            ),
            Style::default(),
        )?;
        acc.gbbox.y2 = bbox.y2 + hq_len;
    }
    if md.task_force {
        let width = match md.echelon.as_deref() {
            Some("Corps/MEF") => 110.0,
            Some("Army") => 145.0,
            Some("Army Group/front") => 180.0,
            Some("Region/Theater") => 215.0,
            _ => 90.0,
        };
        let (l, r) = (100.0 - width / 2.0, 100.0 + width / 2.0);
        let d = alloc::format!(
            "M{},{} L{},{} {},{} {},{}",
            n(l),
            n(bbox.y1),
            n(l),
            n(bbox.y1 - 40.0),
            n(r),
            n(bbox.y1 - 40.0),
            n(r),
            n(bbox.y1)
        );
        acc.push_leaf(s, d, Style::default())?;
        acc.gbbox.x1 = js::min(bbox.x1, l);
        acc.gbbox.x2 = js::max(bbox.x2, r);
        acc.gbbox.y1 = bbox.y1 - 40.0;
    }
    if md.installation {
        installation(s, &mut acc, &bbox, &dim_aff)?;
    }
    if md.flags.feint_dummy == Some(true) {
        let top = bbox.y1 - 0.0 - bbox.width() / 2.0;
        let d = alloc::format!(
            "M100,{} L{},{} M100,{} L{},{}",
            n(top),
            n(bbox.x1),
            n(bbox.y1 - 0.0),
            n(top),
            n(bbox.x2),
            n(bbox.y1 - 0.0)
        );
        let dash = Style {
            stroke_dasharray: Some(Cow::Owned(s.config.dash_arrays.feint_dummy.clone())),
            ..Style::default()
        };
        acc.push_leaf(s, d, dash)?;
        acc.gbbox.merge(PartialBBox {
            y1: Some(top),
            ..PartialBBox::default()
        });
    }
    if md.echelon.as_deref().is_some_and(|e| !e.is_empty()) {
        amplifiers::echelon(s, &mut acc, &bbox)?;
    }
    if md.mobility.as_deref().is_some_and(|m| !m.is_empty()) {
        if !st.frame {
            bbox.y2 = s.bbox.y2;
        }
        amplifiers::mobility(s, &mut acc, &mut bbox)?;
    }
    if md
        .flags
        .leadership
        .as_deref()
        .is_some_and(|l| !l.is_empty())
        && md.affiliation.as_deref() == Some("Friend")
    {
        let style = acc.defaults(Style::default());
        acc.pre.push(Node::Path(PathNode {
            d: PathData::new("m 45,60 55,-25 55,25"),
            style,
        }));
        acc.gbbox.merge(PartialBBox {
            y1: Some(bbox.y1 - 20.0),
            ..PartialBBox::default()
        });
    }
    Ok(PartOutput::new(acc.pre, acc.post, acc.gbbox))
}

fn installation(
    s: &SymbolState<'_>,
    acc: &mut Acc,
    bbox: &BBox,
    dim_aff: &str,
) -> Result<(), RenderError> {
    let sw = s.options.style.stroke_width;
    let gap = if ["AirHostile", "GroundHostile", "SeaHostile"].contains(&dim_aff) {
        14.0
    } else if [
        "AirUnknown",
        "GroundUnknown",
        "SeaUnknown",
        "AirFriend",
        "SeaFriend",
    ]
    .contains(&dim_aff)
    {
        2.0
    } else {
        0.0
    };
    let d = alloc::format!(
        "M85,{} 85,{} 115,{} 115,{} 100,{} Z",
        n(bbox.y1 + gap - sw / 2.0),
        n(bbox.y1 - 10.0),
        n(bbox.y1 - 10.0),
        n(bbox.y1 + gap - sw / 2.0),
        n(bbox.y1 - sw)
    );
    // `fill: color` is set even when the colour is undefined, so the default
    // `fill = false` never applies here.
    let fill = acc.color.clone();
    acc.push_leaf(s, d, Style::default())?;
    if let Some(st) = acc.post.last_mut().and_then(Node::style_mut) {
        st.fill = fill;
    }
    acc.gbbox.merge(PartialBBox {
        y1: Some(bbox.y1 - 10.0),
        ..PartialBBox::default()
    });
    Ok(())
}

//! Instantiates template entries for one symbol: colour slots, dash
//! arrays, extension parts and in-place part mutations.

use super::{ABSENT, IconContext, Kids, Slot, TAff, TDash, TNode, TNum, TPaint};
use crate::color::{ColorSet, SlotMode};
use crate::generated::{pool, tables};
use crate::ir::{self, Node, Num, Paint, Style};
use crate::registry::{IconExtension, IconPartContext, PartLookup};
use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Extensions consulted before the generated tables.
#[derive(Clone, Copy)]
pub(crate) struct UserParts<'a> {
    pub extensions: &'a [Box<dyn IconExtension>],
    pub ctx: Option<&'a IconPartContext<'a>>,
}

/// Resolves symbolic template values.
#[derive(Clone, Copy)]
pub(crate) struct Resolver<'a> {
    pub colors: &'a ColorSet,
    /// Affiliation the icon parts were built for (after `|| "Friend"`).
    pub part_affiliation: Option<crate::domain::Affiliation>,
    pub mono_color: &'a str,
    pub dash_pending: &'a str,
    pub dash_anticipated: &'a str,
    /// Mapping whose in-place part mutations apply (symbol set, or -1 for letter).
    pub mapping: i16,
    pub ctx: IconContext,
    /// Extensions whose parts take precedence over the generated tables
    /// everywhere a part is referenced.
    pub user: UserParts<'a>,
}

fn num(t: TNum) -> Num {
    match t {
        TNum::N(n) => Num::Number(n),
        TNum::S(s) => Num::Text(Cow::Borrowed(s)),
        TNum::B(b) => Num::Bool(b),
    }
}

impl Resolver<'_> {
    fn slot_mode(&self, slot: Slot) -> &SlotMode {
        match slot {
            Slot::Fill => &self.colors.fill_color,
            Slot::Frame => &self.colors.frame_color,
            Slot::Icon => &self.colors.icon_color,
            Slot::IconFill => &self.colors.icon_fill_color,
            Slot::Black => &self.colors.black,
            Slot::White => &self.colors.white,
            Slot::None => &self.colors.none,
        }
    }

    fn paint(&self, p: TPaint) -> Option<Paint> {
        match p {
            TPaint::None => Some(Paint::None),
            TPaint::Lit(s) => Some(Paint::Color(Cow::Borrowed(s))),
            TPaint::Mono => Some(Paint::Color(Cow::Owned(String::from(self.mono_color)))),
            TPaint::Slot(slot, aff) => {
                let mode = self.slot_mode(slot);
                use crate::domain::Affiliation;
                match aff {
                    TAff::SelfAff => self
                        .part_affiliation
                        .and_then(|a| mode.for_affiliation(a))
                        .cloned(),
                    TAff::Civilian => mode.civilian.clone(),
                    TAff::Friend => mode.for_affiliation(Affiliation::Friend).cloned(),
                    TAff::Hostile => mode.for_affiliation(Affiliation::Hostile).cloned(),
                    TAff::Neutral => mode.for_affiliation(Affiliation::Neutral).cloned(),
                    TAff::Unknown => mode.for_affiliation(Affiliation::Unknown).cloned(),
                    TAff::Suspect => mode.suspect.clone(),
                }
            }
        }
    }

    fn style(&self, index: u16) -> Style {
        let Some(t) = pool::STYLES.get(usize::from(index)) else {
            return Style::default();
        };
        Style {
            fill: t.fill.and_then(|p| self.paint(p)),
            fill_opacity: t.fill_opacity.map(num),
            stroke: t.stroke.and_then(|p| self.paint(p)),
            stroke_width: t.stroke_width.map(num),
            stroke_dasharray: t.dash.map(|d| match d {
                TDash::Lit(s) => Cow::Borrowed(s),
                TDash::Pending => Cow::Owned(String::from(self.dash_pending)),
                TDash::Anticipated => Cow::Owned(String::from(self.dash_anticipated)),
            }),
            line_cap: t.line_cap.map(Cow::Borrowed),
            non_scaling_stroke: t.non_scaling_stroke.map(|n| num(n).value()),
            style_fill: t.style_fill,
            icon: t.icon,
            clip_path: t.clip_path.map(Cow::Borrowed),
        }
    }

    fn kids(&self, k: Kids) -> Vec<Node> {
        let start = k.start as usize;
        let end = start.saturating_add(k.len as usize);
        pool::KIDS
            .get(start..end)
            .unwrap_or(&[])
            .iter()
            .map(|&i| self.node(i))
            .collect()
    }

    /// Instantiates pool node `index`.
    pub(crate) fn node(&self, index: u32) -> Node {
        let Some(t) = pool::NODES.get(index as usize) else {
            return Node::Missing;
        };
        match *t {
            TNode::Path { d, style } => Node::Path(ir::PathNode {
                d: ir::PathData::new(d),
                style: self.style(style),
            }),
            TNode::Circle { cx, cy, r, style } => Node::Circle(ir::CircleNode {
                cx: num(cx),
                cy: num(cy),
                r: num(r),
                style: self.style(style),
            }),
            TNode::Text { text, style } => self.text(text, style),
            TNode::Translate { x, y, kids, style } => Node::Translate(ir::TranslateNode {
                x: num(x),
                y: num(y),
                draw: self.kids(kids),
                style: self.style(style),
            }),
            TNode::Rotate {
                degree,
                x,
                y,
                kids,
                style,
            } => Node::Rotate(ir::RotateNode {
                degree: num(degree),
                x: num(x),
                y: num(y),
                draw: self.kids(kids),
                style: self.style(style),
            }),
            TNode::Scale {
                factor,
                kids,
                style,
            } => Node::Scale(ir::ScaleNode {
                factor: num(factor),
                draw: self.kids(kids),
                style: self.style(style),
            }),
            TNode::Group(k) => Node::Group(self.kids(k)),
            TNode::Missing => Node::Missing,
            TNode::Scalar(n) => Node::Scalar(num(n)),
            TNode::Ref(part) => self.part_by_index(part).unwrap_or(Node::Missing),
        }
    }

    fn text(&self, index: u32, style: u16) -> Node {
        let Some(t) = pool::TEXTS.get(index as usize) else {
            return Node::Missing;
        };
        let b = |s: Option<&'static str>| s.map(Cow::Borrowed);
        Node::Text(ir::TextNode {
            x: num(t.x),
            y: num(t.y),
            text: Cow::Borrowed(t.text),
            font_size: t.size.map(num),
            font_family: b(t.family),
            font_weight: b(t.weight),
            text_anchor: b(t.anchor),
            alignment_baseline: b(t.baseline),
            style: self.style(style),
        })
    }

    /// Instantiates a part (honouring in-place mutations of the current mapping).
    fn part_by_index(&self, part: u16) -> Option<Node> {
        let index = usize::from(part);
        let builtin = tables::PARTS.get(index);
        let name = builtin.map(|&(n, _)| n).or_else(|| {
            tables::EXTRA_PART_NAMES
                .get(index.checked_sub(tables::PARTS.len())?)
                .copied()
        })?;
        if let Some(user) = self.user_part(name) {
            return Some(user);
        }
        builtin?;
        let entry = tables::OVERRIDES
            .iter()
            .find(|&&(m, p, _)| m == self.mapping && p == part)
            .map(|&(_, _, e)| e)
            .or_else(|| tables::PARTS.get(usize::from(part)).map(|&(_, e)| e))?;
        self.value(entry)
    }

    /// Instantiates the value of `entry` in the current context.
    pub(crate) fn value(&self, entry: u32) -> Option<Node> {
        match self.ctx.row(entry) {
            ABSENT => None,
            row => Some(self.node(row)),
        }
    }

    /// The part `name` as the latest extension defining it returns it; each
    /// extension sees the parts defined before it.
    fn user_part(&self, name: &str) -> Option<Node> {
        let ctx = self.user.ctx?;
        let exts = self.user.extensions;
        (0..exts.len()).rev().find_map(|i| {
            let earlier = Resolver {
                user: UserParts {
                    extensions: exts.get(..i)?,
                    ctx: Some(ctx),
                },
                ..*self
            };
            exts.get(i)?.icon_part(ctx, name, &earlier)
        })
    }

    /// Instantiates a named icon part (extension parts first).
    pub(crate) fn part(&self, name: &str) -> Option<Node> {
        if let Some(user) = self.user_part(name) {
            return Some(user);
        }
        let idx = tables::PARTS
            .binary_search_by(|(n, _)| n.as_bytes().cmp(name.as_bytes()))
            .ok()?;
        self.part_by_index(u16::try_from(idx).ok()?)
    }

    /// Special bounding box of `entry` in the current context.
    pub(crate) fn bbox(&self, entry: u32) -> Option<[Option<f64>; 4]> {
        match self.ctx.row(entry) {
            ABSENT => None,
            row => tables::BBOXES.get(row as usize).copied(),
        }
    }
}

impl PartLookup for Resolver<'_> {
    fn part(&self, name: &str) -> Option<Node> {
        Resolver::part(self, name)
    }
}

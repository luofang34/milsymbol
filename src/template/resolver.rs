//! Instantiates template entries for one symbol: colour slots, dash
//! arrays, extension parts and in-place part mutations.

use super::{ABSENT, IconContext, Kids, Slot, TAff, TDash, TNode, TNum, TPaint, node_at};
use crate::color::{ColorSet, SlotMode};
use crate::generated::{pool, tables};
use crate::ir::{self, Node, Num, Paint, Str, Style};
use crate::node_list::NodeAcc;
use crate::options::Color;
use crate::registry::{IconExtension, IconPartContext, PartLookup};
use alloc::borrow::Cow;
use alloc::boxed::Box;
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
    pub mono_color: Option<&'a Color>,
    pub icon_font_family: Option<&'a Str>,
    pub dash_pending: &'a Str,
    pub dash_anticipated: &'a Str,
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
            TPaint::Mono => Some(Paint::Color(
                self.mono_color.map_or(Cow::Borrowed(""), Color::to_str),
            )),
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
                TDash::Pending => self.dash_pending.clone(),
                TDash::Anticipated => self.dash_anticipated.clone(),
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
            .map(|&i| self.node(u32::from(i)))
            .collect()
    }

    /// Instantiates pool node `index`.
    pub(crate) fn node(&self, index: u32) -> Node {
        let Some(t) = node_at(index) else {
            return Node::Missing;
        };
        match t {
            #[cfg(not(feature = "compact-paths"))]
            TNode::Path { d, style } => Node::Path(ir::PathNode {
                d: ir::PathData::new(d),
                style: self.style(style),
            }),
            #[cfg(feature = "compact-paths")]
            TNode::Path { d, style } => match super::path_bytes(d) {
                Some(bytes) => Node::Path(ir::PathNode {
                    d: ir::PathData::from_packed(bytes),
                    style: self.style(style),
                }),
                None => Node::Missing,
            },
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
            font_family: self.icon_font_family.cloned().or_else(|| b(t.family)),
            font_weight: b(t.weight),
            text_anchor: b(t.anchor),
            alignment_baseline: b(t.baseline),
            style: self.style(style),
        })
    }

    /// Built-in pool row for `part` when no user extensions are registered.
    fn builtin_part_row(&self, part: u16) -> Option<u32> {
        let (_, default_entry) = *tables::PARTS.get(usize::from(part))?;
        let entry = tables::OVERRIDES
            .iter()
            .find(|&&(m, p, _)| m == self.mapping && p == part)
            .map_or(default_entry, |&(_, _, e)| e);
        match self.ctx.row(entry) {
            ABSENT => None,
            row => Some(row),
        }
    }

    /// Instantiates a part (honouring in-place mutations of the current mapping).
    fn part_by_index(&self, part: u16) -> Option<Node> {
        if self.user.ctx.is_none() {
            return Some(self.node(self.builtin_part_row(part)?));
        }
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
        Some(self.node(self.builtin_part_row(part)?))
    }

    /// Instantiates the value of `entry` in the current context.
    pub(crate) fn value(&self, entry: u32) -> Option<Node> {
        match self.ctx.row(entry) {
            ABSENT => None,
            row => Some(self.node(row)),
        }
    }

    /// Instantiates `entry` directly into `dst`, peeling outer [`TNode::Ref`]s
    /// and single-child [`TNode::Group`]s in place without allocating an
    /// intermediate `Vec<Node>`. Caller must ensure `self.user.ctx.is_none()`.
    pub(crate) fn value_unwrapped(
        &self,
        entry: u32,
        defined_only: bool,
        dst: &mut NodeAcc,
    ) -> Result<bool, crate::error::RenderError> {
        let mut row = self.ctx.row(entry);
        if row == ABSENT {
            return Ok(false);
        }
        while let Some(TNode::Ref(part)) = node_at(row) {
            match self.builtin_part_row(part) {
                Some(next) => row = next,
                None => {
                    if defined_only {
                        return Ok(false);
                    }
                    dst.push(Node::Missing);
                    return Ok(true);
                }
            }
        }
        if defined_only && matches!(node_at(row), None | Some(TNode::Missing)) {
            return Ok(false);
        }
        loop {
            let Some(t) = node_at(row) else {
                dst.push(Node::Missing);
                return Ok(true);
            };
            match t {
                TNode::Ref(part) => match self.builtin_part_row(part) {
                    Some(next) => row = next,
                    None => {
                        dst.push(Node::Missing);
                        return Ok(true);
                    }
                },
                TNode::Group(k) if k.len == 1 => match pool::KIDS.get(k.start as usize) {
                    Some(&child) => row = u32::from(child),
                    None => return Ok(true),
                },
                TNode::Group(k) => {
                    let start = k.start as usize;
                    let end = start.saturating_add(k.len as usize);
                    for &i in pool::KIDS.get(start..end).unwrap_or(&[]) {
                        dst.push(self.node(u32::from(i)));
                    }
                    return Ok(true);
                }
                TNode::Scalar(TNum::S(s)) => {
                    if crate::js::utf16_len(s) == 1 {
                        return Err(crate::error::RenderError::upstream(
                            "single-character string instruction never terminates",
                        ));
                    }
                    if !s.is_empty() {
                        dst.push(Node::Scalar(Num::Text(Cow::Borrowed(s))));
                    }
                    return Ok(true);
                }
                _ => {
                    dst.push(self.node(row));
                    return Ok(true);
                }
            }
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
        match tables::PARTS.binary_search_by(|(n, _)| n.as_bytes().cmp(name.as_bytes())) {
            Ok(idx) => self.part_by_index(u16::try_from(idx).ok()?),
            Err(_) => self.user_part(name),
        }
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

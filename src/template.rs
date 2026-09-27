//! Types for the generated icon tables and their instantiation into IR nodes.
//!
//! Every generated entry (an icon part, a SIDC→icon mapping, a special
//! bounding box) holds one value per combination of the context variables it
//! depends on. [`IconContext`] selects the row; [`Resolver`] turns symbolic
//! colour/dash slots into concrete values.

use crate::color::{ColorSet, SlotMode};
use crate::generated::{pool, tables, vars};
use crate::ir::{self, Node, Num, Paint, Style};
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

/// Number of context variables.
pub(crate) const VAR_COUNT: usize = 15;

/// Row value meaning "upstream defines nothing in this context".
pub(crate) const ABSENT: u32 = u32::MAX;

/// Reference to a part name that upstream never defines.
pub(crate) const UNKNOWN_PART: u16 = u16::MAX;

/// Context variables, in generated-table order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Var {
    Std2525 = 0,
    Frame,
    NumberSidc,
    AlternateMedal,
    Mono,
    Edition,
    NotPresent,
    Affiliation,
    Geometry,
    SlotFill,
    SlotFrame,
    SlotIcon,
    SlotIconFill,
    SlotBlack,
    SlotWhite,
}

/// A numeric template value.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TNum {
    N(f64),
    S(&'static str),
    B(bool),
}

/// Colour slots of [`ColorSet`].
// Variants unused by the current tables stay: regenerated tables may use them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Slot {
    Fill,
    Frame,
    Icon,
    IconFill,
    Black,
    White,
    None,
}

/// Which affiliation's colour a slot reference reads.
// Variants unused by the current tables stay: regenerated tables may use them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TAff {
    SelfAff,
    Civilian,
    Friend,
    Hostile,
    Neutral,
    Unknown,
    Suspect,
}

/// A paint template.
// Variants unused by the current tables stay: regenerated tables may use them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TPaint {
    None,
    Lit(&'static str),
    Slot(Slot, TAff),
    Mono,
}

/// A dash-array template.
// Variants unused by the current tables stay: regenerated tables may use them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TDash {
    Lit(&'static str),
    Pending,
    Anticipated,
}

/// Presentation attributes template.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TStyle {
    pub fill: Option<TPaint>,
    pub fill_opacity: Option<TNum>,
    pub stroke: Option<TPaint>,
    pub stroke_width: Option<TNum>,
    pub dash: Option<TDash>,
    pub line_cap: Option<&'static str>,
    pub non_scaling_stroke: Option<TNum>,
    pub style_fill: Option<bool>,
    pub icon: Option<bool>,
    pub clip_path: Option<&'static str>,
}

/// Text payload template.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TText {
    pub x: TNum,
    pub y: TNum,
    pub text: &'static str,
    pub size: Option<TNum>,
    pub family: Option<&'static str>,
    pub weight: Option<&'static str>,
    pub anchor: Option<&'static str>,
    pub baseline: Option<&'static str>,
}

/// A range of child indices in `KIDS`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Kids {
    pub start: u32,
    pub len: u32,
}

/// A node template.
// Variants unused by the current tables stay: regenerated tables may use them.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum TNode {
    Path {
        d: &'static str,
        style: u16,
    },
    Circle {
        cx: TNum,
        cy: TNum,
        r: TNum,
        style: u16,
    },
    Text {
        text: u32,
        style: u16,
    },
    Translate {
        x: TNum,
        y: TNum,
        kids: Kids,
        style: u16,
    },
    Rotate {
        degree: TNum,
        x: TNum,
        y: TNum,
        kids: Kids,
        style: u16,
    },
    Scale {
        factor: TNum,
        kids: Kids,
        style: u16,
    },
    Group(Kids),
    Missing,
    Scalar(TNum),
    Ref(u16),
}

/// A context-keyed table entry.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub deps: u16,
    pub start: u32,
}

/// Values of the context variables for one symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IconContext {
    values: [u8; VAR_COUNT],
}

impl IconContext {
    pub(crate) fn new() -> Self {
        IconContext {
            values: [0; VAR_COUNT],
        }
    }

    pub(crate) fn set(&mut self, var: Var, value: u8) {
        if let Some(v) = self.values.get_mut(var as usize) {
            *v = value;
        }
    }

    pub(crate) fn set_bool(&mut self, var: Var, value: bool) {
        // Boolean domains are ordered [true, false] except alternateMedal.
        let idx = if var == Var::AlternateMedal {
            u8::from(value)
        } else {
            u8::from(!value)
        };
        self.set(var, idx);
    }

    /// Row value of `entry` in this context.
    fn row(&self, entry: u32) -> u32 {
        let Some(e) = tables::ENTRIES.get(entry as usize) else {
            return ABSENT;
        };
        let mut index: u32 = 0;
        for (vi, (&value, &size)) in self
            .values
            .iter()
            .zip(vars::DOMAIN_SIZES.iter())
            .enumerate()
        {
            if e.deps & (1 << vi) != 0 {
                index = index
                    .saturating_mul(u32::from(size))
                    .saturating_add(u32::from(value));
            }
        }
        e.start
            .checked_add(index)
            .and_then(|row| tables::ROWS.get(row as usize))
            .copied()
            .unwrap_or(ABSENT)
    }
}

/// Resolves symbolic template values.
pub(crate) struct Resolver<'a> {
    pub colors: &'a ColorSet,
    /// Affiliation the icon parts were built for (after `|| "Friend"`).
    pub part_affiliation: &'a str,
    pub mono_color: &'a str,
    pub dash_pending: &'a str,
    pub dash_anticipated: &'a str,
    /// Mapping whose in-place part mutations apply (symbol set, or -1 for letter).
    pub mapping: i16,
    pub ctx: IconContext,
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
                let key = match aff {
                    TAff::SelfAff => self.part_affiliation,
                    TAff::Civilian => "Civilian",
                    TAff::Friend => "Friend",
                    TAff::Hostile => "Hostile",
                    TAff::Neutral => "Neutral",
                    TAff::Unknown => "Unknown",
                    TAff::Suspect => "Suspect",
                };
                mode.get(key)
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
        if part == UNKNOWN_PART {
            return None;
        }
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

    /// Instantiates a named icon part.
    pub(crate) fn part(&self, name: &str) -> Option<Node> {
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

/// Looks up `code` in a byte-sorted `(code, entry)` table.
pub(crate) fn find(table: &[(&str, u32)], code: &str) -> Option<u32> {
    table
        .binary_search_by(|(k, _)| k.as_bytes().cmp(code.as_bytes()))
        .ok()
        .and_then(|i| table.get(i))
        .map(|&(_, e)| e)
}

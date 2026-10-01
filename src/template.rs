//! Types for the generated icon tables and their instantiation into IR nodes.
//!
//! Every generated entry (an icon part, a SIDC→icon mapping, a special
//! bounding box) holds one value per combination of the context variables it
//! depends on. [`IconContext`] selects the row; [`Resolver`] turns symbolic
//! colour/dash slots into concrete values.

use crate::generated::{pool, tables, vars};

/// Number of context variables.
pub(crate) const VAR_COUNT: usize = 15;

/// Row value meaning "upstream defines nothing in this context".
pub(crate) const ABSENT: u32 = u32::MAX;

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
///
/// With `bits == 0` the entry has one value, held in `a`. Otherwise `a` is
/// the start of its palette in `PALETTE` and `b` the byte offset of its
/// bit-packed palette indices in `ROW_DATA`.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Entry {
    pub deps: u16,
    pub bits: u8,
    pub a: u32,
    pub b: u32,
}

/// Packed node record: `PNode(tag, a, b)`, interpreted per tag.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PNode(pub u8, pub u32, pub u16);

/// `PNode` tags.
pub(crate) const T_PATH: u8 = 0;
pub(crate) const T_TEXT: u8 = 1;
pub(crate) const T_GROUP: u8 = 2;
pub(crate) const T_REF: u8 = 3;
pub(crate) const T_WIDE: u8 = 4;

/// Palette value meaning "upstream defines nothing in this context".
const PALETTE_ABSENT: u16 = u16::MAX;

/// Decodes packed node `index`.
pub(crate) fn node_at(index: u32) -> Option<TNode> {
    let &PNode(tag, a, b) = pool::NODES.get(index as usize)?;
    Some(match tag {
        T_PATH => TNode::Path {
            d: pool::PATHS.get(a as usize).copied()?,
            style: b,
        },
        T_TEXT => TNode::Text { text: a, style: b },
        T_GROUP => TNode::Group(Kids {
            start: a,
            len: u32::from(b),
        }),
        T_REF => TNode::Ref(u16::try_from(a).ok()?),
        _ => pool::WIDE.get(a as usize).copied()?,
    })
}

/// `bits` bits (at most 16) of `data` starting at bit `pos`, LSB first.
fn read_bits(data: &[u8], pos: usize, bits: u32) -> Option<u32> {
    let first = pos / 8;
    let mut acc: u32 = 0;
    for k in 0..3 {
        acc |= u32::from(data.get(first + k).copied().unwrap_or(0)) << (8 * k);
    }
    Some((acc >> (pos % 8)) & ((1u32 << bits) - 1))
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
        if e.bits == 0 {
            return e.a;
        }
        let bit = (e.b as usize)
            .saturating_mul(8)
            .saturating_add((index as usize).saturating_mul(usize::from(e.bits)));
        read_bits(&tables::ROW_DATA, bit, u32::from(e.bits))
            .and_then(|slot| tables::PALETTE.get((e.a as usize).checked_add(slot as usize)?))
            .map_or(ABSENT, |&v| {
                if v == PALETTE_ABSENT {
                    ABSENT
                } else {
                    u32::from(v)
                }
            })
    }
}

mod resolver;
pub(crate) use resolver::{Resolver, UserParts};

/// Looks up `code` in a byte-sorted `(code, entry)` table.
pub(crate) fn find(table: &[(&str, u32)], code: &str) -> Option<u32> {
    table
        .binary_search_by(|(k, _)| k.as_bytes().cmp(code.as_bytes()))
        .ok()
        .and_then(|i| table.get(i))
        .map(|&(_, e)| e)
}

//! Types for the generated icon tables and their instantiation into IR nodes.
//!
//! Every generated entry (an icon part, a SIDC→icon mapping, a special
//! bounding box) holds one value per combination of the context variables it
//! depends on. [`IconContext`] selects the row; [`Resolver`] turns symbolic
//! colour/dash slots into concrete values.

use crate::generated::{tables, vars};

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

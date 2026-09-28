//! Colour modes and per-symbol colour resolution (upstream `getColors`).

use crate::domain::Affiliation;
use crate::ir::Paint;
use alloc::borrow::Cow;
use alloc::string::String;

/// Keys of a colour mode, in upstream order.
pub const COLOR_KEYS: [&str; 6] = [
    "Civilian", "Friend", "Hostile", "Neutral", "Unknown", "Suspect",
];

/// A colour per affiliation.
///
/// Each value is `Some(Paint::Color)`, `Some(Paint::None)` (upstream `false`)
/// or `None` (upstream `undefined`, e.g. a key missing from a user object).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorMode {
    /// Civilian colour.
    pub civilian: Option<Paint>,
    /// Friend colour.
    pub friend: Option<Paint>,
    /// Hostile colour.
    pub hostile: Option<Paint>,
    /// Neutral colour.
    pub neutral: Option<Paint>,
    /// Unknown colour.
    pub unknown: Option<Paint>,
    /// Suspect colour.
    pub suspect: Option<Paint>,
}

/// A colour mode slot inside a [`ColorSet`].
pub type SlotMode = ColorMode;

impl ColorMode {
    /// Borrows the paint for a frame affiliation, without allocating.
    /// `None` is an unset slot; `Some(Paint::None)` explicitly disables paint.
    /// Civilian, joker/faker and suspect substitutions are already applied
    /// in the colour modes returned by [`crate::Symbol::colors`].
    pub fn for_affiliation(&self, affiliation: Affiliation) -> Option<&Paint> {
        match affiliation {
            Affiliation::Friend => self.friend.as_ref(),
            Affiliation::Hostile => self.hostile.as_ref(),
            Affiliation::Neutral => self.neutral.as_ref(),
            Affiliation::Unknown => self.unknown.as_ref(),
        }
    }

    /// Builds a mode from colour strings in [`COLOR_KEYS`] order.
    pub fn new(
        civilian: &str,
        friend: &str,
        hostile: &str,
        neutral: &str,
        unknown: &str,
        suspect: &str,
    ) -> Self {
        let c = |s: &str| Some(Paint::Color(Cow::Owned(String::from(s))));
        ColorMode {
            civilian: c(civilian),
            friend: c(friend),
            hostile: c(hostile),
            neutral: c(neutral),
            unknown: c(unknown),
            suspect: c(suspect),
        }
    }

    /// A mode with every affiliation set to `paint`.
    pub fn uniform(paint: Option<Paint>) -> Self {
        ColorMode {
            civilian: paint.clone(),
            friend: paint.clone(),
            hostile: paint.clone(),
            neutral: paint.clone(),
            unknown: paint.clone(),
            suspect: paint,
        }
    }

    pub(crate) fn from_static(values: &[Option<&'static str>; 6]) -> Self {
        let p = |v: Option<&'static str>| {
            Some(v.map_or(Paint::None, |s| Paint::Color(Cow::Borrowed(s))))
        };
        let [c, f, h, n, u, s] = *values;
        ColorMode {
            civilian: p(c),
            friend: p(f),
            hostile: p(h),
            neutral: p(n),
            unknown: p(u),
            suspect: p(s),
        }
    }

    /// Value for an affiliation key (upstream `mode[key]`); unknown keys give `None`.
    pub fn get(&self, key: &str) -> Option<Paint> {
        self.slot(key).and_then(Clone::clone)
    }

    /// Slot for a key.
    pub fn slot(&self, key: &str) -> Option<&Option<Paint>> {
        Some(match key {
            "Civilian" => &self.civilian,
            "Friend" => &self.friend,
            "Hostile" => &self.hostile,
            "Neutral" => &self.neutral,
            "Unknown" => &self.unknown,
            "Suspect" => &self.suspect,
            _ => return None,
        })
    }

    /// Mutable slot for a key.
    pub fn slot_mut(&mut self, key: &str) -> Option<&mut Option<Paint>> {
        Some(match key {
            "Civilian" => &mut self.civilian,
            "Friend" => &mut self.friend,
            "Hostile" => &mut self.hostile,
            "Neutral" => &mut self.neutral,
            "Unknown" => &mut self.unknown,
            "Suspect" => &mut self.suspect,
            _ => return None,
        })
    }

    /// Values in [`COLOR_KEYS`] order.
    pub fn values(&self) -> [&Option<Paint>; 6] {
        [
            &self.civilian,
            &self.friend,
            &self.hostile,
            &self.neutral,
            &self.unknown,
            &self.suspect,
        ]
    }
}

/// JavaScript truthiness of a colour value.
pub(crate) fn truthy(v: &Option<Paint>) -> bool {
    matches!(v, Some(Paint::Color(s)) if !s.is_empty())
}

/// Resolved colours of one symbol (upstream `symbol.colors`).
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct ColorSet {
    /// Frame fill colours.
    pub fill_color: ColorMode,
    /// Frame stroke colours.
    pub frame_color: ColorMode,
    /// Icon stroke colours.
    pub icon_color: ColorMode,
    /// Icon fill colours.
    pub icon_fill_color: ColorMode,
    /// All-`false` mode.
    pub none: ColorMode,
    /// "Black" colours (the frame colour in monochrome mode).
    pub black: ColorMode,
    /// "White" colours.
    pub white: ColorMode,
}

/// Inputs of [`resolve_colors`].
pub(crate) struct ColorInputs<'a> {
    pub fill_mode: ColorMode,
    pub frame_override: Option<&'a ColorMode>,
    pub icon_override: Option<&'a ColorMode>,
    pub frame_mode: ColorMode,
    pub icon_mode: ColorMode,
    pub black: ColorMode,
    pub white: ColorMode,
    pub off_white: ColorMode,
    pub none: ColorMode,
    pub civilian_color: bool,
    pub mono_color: &'a str,
    pub icon_visible: bool,
}

/// Symbol properties [`resolve_colors`] depends on.
pub(crate) struct ColorFlags {
    pub civilian: bool,
    pub joker_or_faker: bool,
    pub suspect: bool,
    pub frame: bool,
    pub fill: bool,
}

/// Upstream mutates the style's colour objects in place; later symbol parts
/// and `getOptions()` observe these values.
pub(crate) struct MutatedStyle {
    /// `style.colorMode` when it is an object.
    pub color_mode: ColorMode,
    /// `style.frameColor` when it is an object.
    pub frame_color: ColorMode,
    /// `style.iconColor` when it is an object.
    pub icon_color: ColorMode,
}

/// Civilian, joker/faker and suspect colour substitutions, applied in place
/// to the fill, frame and icon modes.
fn apply_identity(modes: [&mut ColorMode; 3], civilian: bool, f: &ColorFlags) {
    for m in modes {
        if civilian {
            m.friend = m.civilian.clone();
            m.neutral = m.civilian.clone();
            m.unknown = m.civilian.clone();
        }
        if f.joker_or_faker {
            m.friend = m.hostile.clone();
        }
        if f.suspect {
            m.friend = m.suspect.clone();
            m.hostile = m.suspect.clone();
        }
    }
}

fn monochrome_frame(frame: &mut ColorMode, color: &str) {
    let mono = Some(Paint::Color(Cow::Owned(String::from(color))));
    for slot in [
        &mut frame.friend,
        &mut frame.neutral,
        &mut frame.hostile,
        &mut frame.unknown,
        &mut frame.civilian,
    ] {
        *slot = mono.clone();
    }
}

/// Port of upstream `getColors`.
pub(crate) fn resolve_colors(mut i: ColorInputs<'_>, f: &ColorFlags) -> (ColorSet, MutatedStyle) {
    // Upstream aliases: `baseIconFillColor` is the fill object itself, and
    // user-supplied frame/icon objects are mutated in place below.
    let mut fill = i.fill_mode.clone();
    let frame_is_override = i.frame_override.is_some();
    let icon_is_override = i.icon_override.is_some();
    let mut frame = i
        .frame_override
        .cloned()
        .unwrap_or_else(|| i.frame_mode.clone());
    let mut icon = i
        .icon_override
        .cloned()
        .unwrap_or_else(|| i.icon_mode.clone());
    apply_identity(
        [&mut fill, &mut frame, &mut icon],
        i.civilian_color && f.civilian,
        f,
    );
    let fill_object = fill.clone();
    if !i.mono_color.is_empty() {
        monochrome_frame(&mut frame, i.mono_color);
        i.black = frame.clone();
        i.white = i.none.clone();
        fill = i.none.clone();
    }
    let frame_color = match (f.frame, frame_is_override) {
        (false, _) => i.none.clone(),
        (true, true) => frame.clone(),
        (true, false) => i.black.clone(),
    };
    let colors = if f.fill {
        ColorSet {
            fill_color: if !f.frame && i.icon_visible {
                i.none.clone()
            } else {
                fill.clone()
            },
            frame_color,
            icon_color: if icon_is_override {
                icon.clone()
            } else {
                i.black.clone()
            },
            icon_fill_color: if f.frame { i.off_white.clone() } else { fill },
            none: i.none,
            black: i.black,
            white: i.off_white,
        }
    } else {
        let blank = !f.frame && !i.icon_visible;
        ColorSet {
            fill_color: if blank {
                i.black.clone()
            } else {
                i.none.clone()
            },
            frame_color: if blank {
                i.black.clone()
            } else if f.frame {
                frame.clone()
            } else {
                i.none.clone()
            },
            icon_color: frame.clone(),
            icon_fill_color: i.none.clone(),
            none: i.none,
            black: i.black,
            white: i.white,
        }
    };
    let mutated = MutatedStyle {
        color_mode: fill_object,
        frame_color: frame,
        icon_color: icon,
    };
    (colors, mutated)
}

#[cfg(test)]
mod tests;

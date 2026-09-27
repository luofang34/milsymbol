//! Colour modes and per-symbol colour resolution (upstream `getColors`).

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

fn copy_to(mode: &mut ColorMode, from: &str, to: &[&str]) {
    let v = mode.get(from);
    for k in to {
        if let Some(s) = mode.slot_mut(k) {
            *s = v.clone();
        }
    }
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

/// Port of upstream `getColors`.
pub(crate) fn resolve_colors(mut i: ColorInputs<'_>, f: &ColorFlags) -> (ColorSet, MutatedStyle) {
    // Upstream aliases: `baseIconFillColor` is the fill object itself, and
    // user-supplied frame/icon objects are mutated in place below.
    let mut fill = i.fill_mode;
    let frame_is_override = i.frame_override.is_some();
    let icon_is_override = i.icon_override.is_some();
    let mut frame = i.frame_override.cloned().unwrap_or(i.frame_mode);
    let mut icon = i.icon_override.cloned().unwrap_or(i.icon_mode);
    if i.civilian_color && f.civilian {
        for m in [&mut fill, &mut frame, &mut icon] {
            copy_to(m, "Civilian", &["Friend", "Neutral", "Unknown"]);
        }
    }
    if f.joker_or_faker {
        for m in [&mut fill, &mut frame, &mut icon] {
            copy_to(m, "Hostile", &["Friend"]);
        }
    }
    if f.suspect {
        for m in [&mut fill, &mut frame, &mut icon] {
            copy_to(m, "Suspect", &["Friend", "Hostile"]);
        }
    }
    let icon_fill = fill.clone();
    let fill_object = fill.clone();
    if !i.mono_color.is_empty() {
        let mono = Some(Paint::Color(Cow::Owned(String::from(i.mono_color))));
        for k in ["Friend", "Neutral", "Hostile", "Unknown", "Civilian"] {
            if let Some(s) = frame.slot_mut(k) {
                *s = mono.clone();
            }
        }
        i.black = frame.clone();
        i.white = i.none.clone();
        fill = i.none.clone();
    }
    let mut colors = ColorSet {
        fill_color: fill.clone(),
        frame_color: frame.clone(),
        icon_color: icon.clone(),
        icon_fill_color: icon_fill,
        none: i.none.clone(),
        black: i.black.clone(),
        white: i.white.clone(),
    };
    colors.frame_color = if f.frame {
        if frame_is_override {
            frame.clone()
        } else {
            i.black.clone()
        }
    } else {
        i.none.clone()
    };
    if f.fill {
        colors.fill_color = if !f.frame && i.icon_visible {
            i.none.clone()
        } else {
            fill.clone()
        };
        colors.icon_color = if icon_is_override {
            icon.clone()
        } else {
            i.black.clone()
        };
        colors.icon_fill_color = if f.frame { i.off_white.clone() } else { fill };
        colors.white = i.off_white;
    } else {
        colors.fill_color = i.none.clone();
        colors.frame_color = if f.frame {
            frame.clone()
        } else {
            i.none.clone()
        };
        colors.icon_color = frame.clone();
        colors.icon_fill_color = i.none.clone();
        if !f.frame && !i.icon_visible {
            colors.frame_color = i.black.clone();
            colors.fill_color = i.black;
        }
    }
    let mutated = MutatedStyle {
        color_mode: fill_object,
        frame_color: frame,
        icon_color: icon,
    };
    (colors, mutated)
}

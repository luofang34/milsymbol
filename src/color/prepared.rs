//! Precomputed colour sets for a [`crate::Renderer`], avoiding per-symbol
//! `BTreeMap` lookups and cloning of built-in colour tables.

use super::{ColorFlags, ColorInputs, ColorMode, ColorSet, resolve_colors};
use crate::config::RendererConfig;
use crate::generated::misc;
use alloc::boxed::Box;
use core::ops::Deref;

/// Resolved colours of a symbol: a static built-in table when a standard
/// framed+filled mode matches unmodified, or an owned [`ColorSet`] otherwise.
#[derive(Debug, Clone)]
pub(crate) enum SymbolColors {
    Static(&'static ColorSet),
    Owned(Box<ColorSet>),
}

impl Deref for SymbolColors {
    type Target = ColorSet;

    fn deref(&self) -> &ColorSet {
        match self {
            SymbolColors::Static(c) => c,
            SymbolColors::Owned(c) => c,
        }
    }
}

/// Base slot modes looked up once from [`RendererConfig::color_modes`].
#[derive(Debug, Clone)]
pub(crate) struct BaseModes {
    pub frame_mode: ColorMode,
    pub icon_mode: ColorMode,
    pub black: ColorMode,
    pub white: ColorMode,
    pub off_white: ColorMode,
    pub none: ColorMode,
}

#[derive(Debug, Clone)]
enum Preset {
    Static(&'static ColorSet),
    Custom(Box<ColorSet>),
}

impl Preset {
    fn to_symbol_colors(&self) -> SymbolColors {
        match self {
            Preset::Static(c) => SymbolColors::Static(c),
            Preset::Custom(c) => SymbolColors::Owned(c.clone()),
        }
    }
}

/// Colour tables prepared once when a [`crate::Renderer`] is built.
#[derive(Debug, Clone)]
pub(crate) struct PreparedColors {
    pub base: Option<BaseModes>,
    light: Option<Preset>,
    medium: Option<Preset>,
    dark: Option<Preset>,
}

impl PreparedColors {
    pub(crate) fn new(config: &RendererConfig) -> Self {
        let base = (|| {
            Some(BaseModes {
                frame_mode: config.color_mode("FrameColor")?.clone(),
                icon_mode: config.color_mode("IconColor")?.clone(),
                black: config.color_mode("Black")?.clone(),
                white: config.color_mode("White")?.clone(),
                off_white: config.color_mode("OffWhite")?.clone(),
                none: config.color_mode("None")?.clone(),
            })
        })();
        let preset = |name: &str, builtin: &'static ColorSet| {
            let b = base.as_ref()?;
            let fill_mode = config.color_mode(name)?.clone();
            let inputs = ColorInputs {
                fill_mode,
                frame_override: None,
                icon_override: None,
                frame_mode: b.frame_mode.clone(),
                icon_mode: b.icon_mode.clone(),
                black: b.black.clone(),
                white: b.white.clone(),
                off_white: b.off_white.clone(),
                none: b.none.clone(),
                civilian_color: true,
                mono_color: "",
                icon_visible: true,
            };
            let flags = ColorFlags {
                civilian: false,
                joker_or_faker: false,
                suspect: false,
                frame: true,
                fill: true,
            };
            let resolved = resolve_colors(inputs, &flags).0;
            Some(if resolved == *builtin {
                Preset::Static(builtin)
            } else {
                Preset::Custom(Box::new(resolved))
            })
        };
        PreparedColors {
            light: preset("Light", &DEFAULT_LIGHT),
            medium: preset("Medium", &DEFAULT_MEDIUM),
            dark: preset("Dark", &DEFAULT_DARK),
            base,
        }
    }

    pub(crate) fn framed_filled(&self, name: &str) -> Option<SymbolColors> {
        let p = match name {
            "Light" => self.light.as_ref()?,
            "Medium" => self.medium.as_ref()?,
            "Dark" => self.dark.as_ref()?,
            _ => return None,
        };
        Some(p.to_symbol_colors())
    }
}

const fn eq_str(a: &str, b: &str) -> bool {
    let (mut rest_a, mut rest_b) = (a.as_bytes(), b.as_bytes());
    if rest_a.len() != rest_b.len() {
        return false;
    }
    while let ([xa, ta @ ..], [xb, tb @ ..]) = (rest_a, rest_b) {
        if *xa != *xb {
            return false;
        }
        rest_a = ta;
        rest_b = tb;
    }
    true
}

const fn builtin_mode(target: &str) -> ColorMode {
    let mut rest: &[(&str, [Option<&str>; 6])] = &misc::COLOR_MODES;
    while let [(name, values), tail @ ..] = rest {
        if eq_str(name, target) {
            return ColorMode::from_static(values);
        }
        rest = tail;
    }
    ColorMode::from_static(&[None; 6])
}

const fn builtin_framed_filled(fill_name: &str) -> ColorSet {
    ColorSet {
        fill_color: builtin_mode(fill_name),
        frame_color: builtin_mode("Black"),
        icon_color: builtin_mode("Black"),
        icon_fill_color: builtin_mode("OffWhite"),
        none: builtin_mode("None"),
        black: builtin_mode("Black"),
        white: builtin_mode("OffWhite"),
    }
}

static DEFAULT_LIGHT: ColorSet = builtin_framed_filled("Light");
static DEFAULT_MEDIUM: ColorSet = builtin_framed_filled("Medium");
static DEFAULT_DARK: ColorSet = builtin_framed_filled("Dark");

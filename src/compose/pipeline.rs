//! The composition driver (upstream `setOptions`, `getMetadata`, `getColors`).

use super::{PartOutput, SymbolState};
use crate::bbox::BBox;
use crate::color::{self, ColorFlags, ColorInputs, ColorMode, ColorSet};
use crate::config::{RendererConfig, Standard};
use crate::error::RenderError;
use crate::geometry;
use crate::ir::{Node, Point};
use crate::js;
use crate::metadata::Metadata;
use crate::options::{StyleColor, SymbolOptions};
use crate::registry::{PartSlot, Registry};
use crate::sidc::{self, Dashes, ParseInput};
use alloc::string::String;
use alloc::vec::Vec;

/// Result of composing one symbol.
pub(crate) struct Composition {
    pub sidc: String,
    pub metadata: Metadata,
    pub colors: ColorSet,
    pub instructions: Vec<Node>,
    pub bbox: BBox,
    pub base_width: f64,
    pub base_height: f64,
    pub width: f64,
    pub height: f64,
    pub anchor: Point,
    pub octagon_anchor: Point,
    pub valid_icon: bool,
}

/// Upstream `getMetadata`.
pub(crate) fn metadata(
    sidc_in: &str,
    options: &SymbolOptions,
    config: &RendererConfig,
) -> (String, Metadata) {
    let st = &options.style;
    let std2525 = if st.standard.is_empty() {
        config.standard == Standard::Mil2525
    } else {
        st.standard != "APP6"
    };
    let mut md = Metadata::new(st.fill, st.frame, std2525);
    if !st.mono_color.is_empty() {
        md.fill = false;
    }
    let normalized = sidc::normalize(sidc_in);
    let input = ParseInput {
        style_frame: st.frame,
        alternate_medal: st.alternate_medal,
        dashes: Dashes {
            pending: &config.dash_arrays.pending,
            anticipated: &config.dash_arrays.anticipated,
        },
    };
    let sidc = sidc::interpret(&normalized, &mut md, &input);
    let name = alloc::format!(
        "{}{}",
        md.dimension,
        md.affiliation.as_deref().unwrap_or("undefined")
    );
    md.base_geometry = geometry::static_name(&name);
    if !st.frame && !st.icon {
        md.base_geometry = geometry::static_name("PositionMarker");
    }
    (sidc, md)
}

fn named_mode(config: &RendererConfig, name: &str) -> Result<ColorMode, RenderError> {
    config
        .color_mode(name)
        .cloned()
        .ok_or_else(|| RenderError::UnknownColorMode {
            name: String::from(name),
        })
}

/// Upstream `getColors`.
pub(crate) fn colors(
    md: &Metadata,
    options: &SymbolOptions,
    config: &RendererConfig,
) -> Result<ColorSet, RenderError> {
    let st = &options.style;
    let fill_mode = match &st.color_mode {
        StyleColor::PerAffiliation(m) => m.clone(),
        StyleColor::Str(name) => named_mode(config, name)?,
    };
    let inputs = ColorInputs {
        fill_mode,
        frame_override: st.frame_color.as_mode(),
        icon_override: st.icon_color.as_mode(),
        frame_mode: named_mode(config, "FrameColor")?,
        icon_mode: named_mode(config, "IconColor")?,
        black: named_mode(config, "Black")?,
        white: named_mode(config, "White")?,
        off_white: named_mode(config, "OffWhite")?,
        none: named_mode(config, "None")?,
        civilian_color: st.civilian_color,
        mono_color: &st.mono_color,
        icon_visible: st.icon,
    };
    let flags = ColorFlags {
        civilian: md.civilian,
        joker_or_faker: md.joker || md.faker,
        suspect: md.flags.suspect == Some(true),
        frame: md.frame,
        fill: md.fill,
    };
    Ok(color::resolve_colors(inputs, &flags))
}

/// A JavaScript value produced by unwrapping single-element arrays.
enum Unwrapped {
    Array(Vec<Node>),
    One(Node),
    Undefined,
}

fn unwrap_value(list: Vec<Node>, is_pre: bool) -> Result<Unwrapped, RenderError> {
    let mut v = Unwrapped::Array(list);
    loop {
        v = match v {
            Unwrapped::Array(mut a) if a.len() == 1 => match a.pop() {
                Some(Node::Group(g)) => Unwrapped::Array(g),
                Some(Node::Missing) | None => {
                    if is_pre {
                        return Err(RenderError::upstream(
                            "Cannot read properties of undefined (reading 'length')",
                        ));
                    }
                    return Ok(Unwrapped::Undefined);
                }
                Some(Node::Scalar(crate::ir::Num::Text(t))) if js::utf16_len(&t) == 1 => {
                    return Err(RenderError::upstream(
                        "single-character string instruction never terminates",
                    ));
                }
                Some(n) => Unwrapped::One(n),
            },
            other => return Ok(other),
        };
    }
}

/// JavaScript `length != 0` of an unwrapped value (`undefined != 0` is true).
fn nonzero_length(v: &Unwrapped) -> bool {
    match v {
        Unwrapped::Array(a) => !a.is_empty(),
        Unwrapped::One(Node::Scalar(crate::ir::Num::Text(t))) => !t.is_empty(),
        Unwrapped::One(_) | Unwrapped::Undefined => true,
    }
}

fn into_nodes(v: Unwrapped) -> Vec<Node> {
    match v {
        Unwrapped::Array(a) => a,
        Unwrapped::One(n) => alloc::vec![n],
        Unwrapped::Undefined => alloc::vec![Node::Missing],
    }
}

/// Merges one part's output into the draw list (upstream `setOptions` loop body).
fn merge(
    out: PartOutput,
    instructions: &mut Vec<Node>,
    bbox: &mut BBox,
) -> Result<(), RenderError> {
    let not_empty = !out.pre.is_empty() || !out.post.is_empty();
    if !out.pre.is_empty() {
        let pre = unwrap_value(out.pre, true)?;
        if nonzero_length(&pre) {
            let mut v = into_nodes(pre);
            v.append(instructions);
            *instructions = v;
        }
    }
    if !out.post.is_empty() {
        let post = unwrap_value(out.post, false)?;
        if nonzero_length(&post) {
            instructions.extend(into_nodes(post));
        }
    }
    if not_empty {
        bbox.merge(out.bbox);
    }
    Ok(())
}

/// Composes a symbol.
pub(crate) fn compose(
    sidc_in: &str,
    options: &SymbolOptions,
    config: &RendererConfig,
    registry: &Registry,
) -> Result<Composition, RenderError> {
    let (sidc, md) = metadata(sidc_in, options, config);
    let colors = colors(&md, options, config)?;
    let mut instructions = Vec::new();
    let mut bbox = BBox::default();
    let mut valid_icon = true;
    for slot in &registry.parts {
        let state = SymbolState {
            sidc: &sidc,
            options,
            metadata: &md,
            colors: &colors,
            bbox,
            config,
            registry,
        };
        let out = match slot {
            PartSlot::Builtin(p) => crate::compose::SymbolPart::draw(p, &state)?,
            PartSlot::Custom(p) => p.draw(&state)?,
        };
        valid_icon &= !out.invalid_icon;
        merge(out, &mut instructions, &mut bbox)?;
    }
    Ok(finish(
        sidc,
        md,
        colors,
        instructions,
        bbox,
        valid_icon,
        options,
        config,
    ))
}

#[allow(clippy::too_many_arguments)]
fn finish(
    sidc: String,
    md: Metadata,
    colors: ColorSet,
    instructions: Vec<Node>,
    mut bbox: BBox,
    valid_icon: bool,
    options: &SymbolOptions,
    config: &RendererConfig,
) -> Composition {
    let st = &options.style;
    let (sw, ow, size) = (st.stroke_width, st.outline_width, st.size);
    if st.padding != 0.0 && !st.padding.is_nan() {
        bbox.x1 -= st.padding;
        bbox.x2 += st.padding;
        bbox.y1 -= st.padding;
        bbox.y2 += st.padding;
    }
    let octagon_anchor = Point {
        x: ((100.0 - bbox.x1 + sw + ow) * size) / 100.0,
        y: ((100.0 - bbox.y1 + sw + ow) * size) / 100.0,
    };
    let mut anchor = Point { x: 100.0, y: 100.0 };
    if md.headquarters {
        let hq = if st.hq_staff_length != 0.0 && !st.hq_staff_length.is_nan() {
            st.hq_staff_length
        } else {
            config.hq_staff_length
        };
        let g = md.geometry_bbox();
        anchor = Point {
            x: g.x1,
            y: g.y2 + hq,
        };
    }
    if st.square {
        let maxx = js::max(anchor.x - bbox.x1, bbox.x2 - anchor.x);
        let maxy = js::max(anchor.y - bbox.y1, bbox.y2 - anchor.y);
        let max = js::max(maxx, maxy);
        bbox = BBox {
            x1: anchor.x - max,
            y1: anchor.y - max,
            x2: anchor.x + max,
            y2: anchor.y + max,
        };
    }
    let base_width = bbox.width() + sw * 2.0 + ow * 2.0;
    let base_height = bbox.height() + sw * 2.0 + ow * 2.0;
    Composition {
        sidc,
        metadata: md,
        colors,
        instructions,
        bbox,
        base_width,
        base_height,
        width: (base_width * size) / 100.0,
        height: (base_height * size) / 100.0,
        anchor: Point {
            x: ((anchor.x - bbox.x1 + sw + ow) * size) / 100.0,
            y: ((anchor.y - bbox.y1 + sw + ow) * size) / 100.0,
        },
        octagon_anchor,
        valid_icon,
    }
}

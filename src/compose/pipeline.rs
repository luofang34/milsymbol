//! The composition driver (upstream `setOptions`, `getMetadata`, `getColors`).

use super::{BuiltinPart, SymbolState};
use crate::bbox::{BBox, PartialBBox};
use crate::color::{self, ColorFlags, ColorInputs, ColorMode, PreparedColors, SymbolColors};
use crate::config::{RendererConfig, Standard};
use crate::error::RenderError;
use crate::ir::{Node, Num, Paint, Point, Style};
use crate::js;
use crate::metadata::Metadata;
use crate::node_list::NodeAcc;
use crate::options::{ColorChoice, ColorModeChoice, SymbolOptions};
use crate::registry::{PartSlot, Registry};
use crate::sidc::{self, Dashes, ParseInput};
use crate::small_str::SmallStr;
use crate::symbol::{Size, Symbol};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// Upstream `getMetadata`.
pub(crate) fn metadata(
    sidc_in: &str,
    options: &SymbolOptions,
    config: &RendererConfig,
) -> (SmallStr<30>, Metadata) {
    let st = &options.style;
    let std2525 = st.standard.unwrap_or(config.standard) == Standard::Mil2525;
    let mut md = Metadata::new(st.fill, st.frame, std2525);
    if st.mono_color.is_some() {
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
    let sidc = sidc::interpret(normalized, &mut md, &input);
    let name = crate::geometry::frame_name(md.dimension.known(), md.affiliation.known());
    md.base_geometry = name;
    if !st.frame && !st.icon {
        md.base_geometry = Some("PositionMarker");
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

/// Upstream `getColors`. Writes back the in-place mutations upstream makes to
/// object-valued `colorMode`, `frameColor` and `iconColor`.
pub(crate) fn colors(
    md: &Metadata,
    options: &mut SymbolOptions,
    config: &RendererConfig,
    prepared: &PreparedColors,
) -> Result<SymbolColors, RenderError> {
    let st = &options.style;
    let frame_override = st.frame_color.as_ref().and_then(ColorChoice::as_mode);
    let icon_override = st.icon_color.as_ref().and_then(ColorChoice::as_mode);
    let unmodified = frame_override.is_none()
        && icon_override.is_none()
        && st.mono_color.is_none()
        && !(st.civilian_color && md.civilian)
        && !(md.joker || md.faker)
        && md.flags.suspect != Some(true)
        && md.frame
        && md.fill;
    if unmodified {
        if let ColorModeChoice::Named(name) = &st.color_mode {
            if let Some(preset) = prepared.framed_filled(name) {
                return Ok(preset);
            }
        }
    }
    let fill_mode = match &st.color_mode {
        ColorModeChoice::Custom(m) => m.clone(),
        ColorModeChoice::Named(name) => named_mode(config, name)?,
    };
    let (frame_mode, icon_mode, black, white, off_white, none) = match &prepared.base {
        Some(b) => (
            b.frame_mode.clone(),
            b.icon_mode.clone(),
            b.black.clone(),
            b.white.clone(),
            b.off_white.clone(),
            b.none.clone(),
        ),
        None => (
            named_mode(config, "FrameColor")?,
            named_mode(config, "IconColor")?,
            named_mode(config, "Black")?,
            named_mode(config, "White")?,
            named_mode(config, "OffWhite")?,
            named_mode(config, "None")?,
        ),
    };
    let inputs = ColorInputs {
        fill_mode,
        frame_override,
        icon_override,
        frame_mode,
        icon_mode,
        black,
        white,
        off_white,
        none,
        civilian_color: st.civilian_color,
        mono_color: st.mono_color_str(),
        icon_visible: st.icon,
    };
    let flags = ColorFlags {
        civilian: md.civilian,
        joker_or_faker: md.joker || md.faker,
        suspect: md.flags.suspect == Some(true),
        frame: md.frame,
        fill: md.fill,
    };
    let (colors, mutated) = color::resolve_colors(inputs, &flags);
    if let ColorModeChoice::Custom(m) = &mut options.style.color_mode {
        *m = mutated.color_mode;
    }
    if let Some(ColorChoice::PerAffiliation(m)) = &mut options.style.frame_color {
        *m = mutated.frame_color;
    }
    if let Some(ColorChoice::PerAffiliation(m)) = &mut options.style.icon_color {
        *m = mutated.icon_color;
    }
    Ok(SymbolColors::Owned(Box::new(colors)))
}

/// A JavaScript value produced by unwrapping single-element arrays.
#[allow(clippy::large_enum_variant)]
enum Unwrapped {
    Array(Vec<Node>),
    One(Node),
    Undefined,
}

fn unwrap_vec(list: Vec<Node>, is_pre: bool) -> Result<Unwrapped, RenderError> {
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

/// Puts `v` before the existing instructions (upstream `pre.concat(draw)`).
fn prepend(v: Unwrapped, acc: &mut NodeAcc) {
    match v {
        Unwrapped::Array(a) => acc.prepend_vec(a),
        Unwrapped::One(n) => acc.prepend_one(n),
        Unwrapped::Undefined => acc.prepend_one(Node::Missing),
    }
}

/// Puts `v` after the existing instructions (upstream `draw.concat(post)`).
fn append(v: Unwrapped, acc: &mut NodeAcc) {
    match v {
        Unwrapped::Array(a) => acc.extend_vec(a),
        Unwrapped::One(n) => acc.push(n),
        Unwrapped::Undefined => acc.push(Node::Missing),
    }
}

/// Merges `pre`/`post` vectors into `acc` (upstream `setOptions` loop body).
pub(super) fn merge_vecs(
    pre: Vec<Node>,
    post: Vec<Node>,
    part_bbox: PartialBBox,
    acc: &mut NodeAcc,
    bbox: &mut BBox,
) -> Result<(), RenderError> {
    let not_empty = !pre.is_empty() || !post.is_empty();
    if !pre.is_empty() {
        let pre = unwrap_vec(pre, true)?;
        if nonzero_length(&pre) {
            prepend(pre, acc);
        }
    }
    if !post.is_empty() {
        let post = unwrap_vec(post, false)?;
        if nonzero_length(&post) {
            append(post, acc);
        }
    }
    if not_empty {
        bbox.merge(part_bbox);
    }
    Ok(())
}

impl BuiltinPart {
    fn render_direct(
        &self,
        s: &SymbolState<'_>,
        acc: &mut NodeAcc,
        bbox: &mut BBox,
        valid_icon: &mut bool,
    ) -> Result<(), RenderError> {
        let out = match self {
            BuiltinPart::BaseGeometry => return super::base_geometry::render_direct(s, acc, bbox),
            BuiltinPart::Icon => return super::icon::render_direct(s, acc, bbox, valid_icon),
            BuiltinPart::Modifier => return super::modifier::render_direct(s, acc, bbox),
            BuiltinPart::Octagon => {
                acc.push(octagon_node());
                bbox.merge(BBox::default().into());
                return Ok(());
            }
            BuiltinPart::Stack if s.options.stack.is_some() => super::stack::draw(s)?,
            BuiltinPart::StatusModifier if s.metadata.condition.is_some() => {
                super::status::draw(s)?
            }
            BuiltinPart::Engagement if !s.options.text.is_empty() => super::engagement::draw(s)?,
            BuiltinPart::AffiliationDimension
                if s.metadata.dimension_unknown
                    || matches!(
                        s.metadata.context,
                        Some(crate::domain::Context::Exercise | crate::domain::Context::Simulation)
                    )
                    || s.options.style.outline_width > 0.0 =>
            {
                super::affiliation::draw(s)?
            }
            BuiltinPart::TextFields => super::textfields::draw(s)?,
            BuiltinPart::DirectionArrow
                if s.options.direction.is_some() && s.options.style.info_fields =>
            {
                super::direction::draw(s)?
            }
            BuiltinPart::Stack
            | BuiltinPart::StatusModifier
            | BuiltinPart::Engagement
            | BuiltinPart::AffiliationDimension
            | BuiltinPart::DirectionArrow => return Ok(()),
        };
        *valid_icon &= !out.invalid_icon;
        merge_vecs(out.pre, out.post, out.bbox, acc, bbox)
    }
}

pub(super) fn octagon_node() -> Node {
    let mut n = crate::ir::PathNode {
        d: crate::ir::PathData::new(
            "m 120,60 0,80 m -40,-80 0,80 m -20,-20 80,0 m 0,-40 -80,0 M 100,50 135.35534,64.64466 150,100 135.35534,135.35534 100,150.00002 64.644661,135.35534 50,100 64.644661,64.64466 z",
        ),
        style: Style::default(),
    };
    n.style.fill = Some(Paint::None);
    n.style.stroke = Some(Paint::color("rgb(0,0,255)"));
    n.style.stroke_width = Some(Num::Number(1.0));
    Node::Path(n)
}

/// Composes a symbol directly into [`Symbol`].
pub(crate) fn compose(
    sidc_in: &str,
    mut options: SymbolOptions,
    config: &RendererConfig,
    prepared: &PreparedColors,
    registry: &Registry,
) -> Result<Symbol, RenderError> {
    options.check_numbers()?;
    let (sidc, md) = metadata(sidc_in, &options, config);
    let colors = colors(&md, &mut options, config, prepared)?;
    let sidc_str = sidc.as_str();
    let mut acc = NodeAcc::default();
    let mut bbox = BBox::default();
    let mut valid_icon = true;
    let mut icon_checked = false;
    for (index, slot) in registry.parts.iter().enumerate() {
        let state = SymbolState {
            sidc: sidc_str,
            options: &options,
            metadata: &md,
            colors: &colors,
            bbox,
            config,
            registry,
        };
        match slot {
            PartSlot::Builtin(p) => {
                icon_checked |= options.style.icon && matches!(p, BuiltinPart::Icon);
                p.render_direct(&state, &mut acc, &mut bbox, &mut valid_icon)?;
            }
            PartSlot::Custom(p) => {
                let out = p
                    .draw(&state)
                    .map_err(|source| RenderError::Part { index, source })?;
                valid_icon &= !out.invalid_icon;
                merge_vecs(out.pre, out.post, out.bbox, &mut acc, &mut bbox)?;
            }
        }
    }
    let icon_known = valid_icon
        && (icon_checked
            || super::icon::known(&SymbolState {
                sidc: sidc_str,
                options: &options,
                metadata: &md,
                colors: &colors,
                bbox,
                config,
                registry,
            }));
    let l = layout(bbox, &md, &options, config);
    let instructions = acc.finish();
    Ok(Symbol {
        sidc,
        options,
        metadata: md,
        colors,
        instructions,
        bbox: l.bbox,
        base_width: l.base_width,
        base_height: l.base_height,
        size: Size {
            width: l.width,
            height: l.height,
        },
        anchor: l.anchor,
        octagon_anchor: l.octagon_anchor,
        valid_icon,
        icon_known,
    })
}

/// Final bounds, size and anchors of a composed symbol.
struct Layout {
    bbox: BBox,
    base_width: f64,
    base_height: f64,
    width: f64,
    height: f64,
    anchor: Point,
    octagon_anchor: Point,
}

/// Padding, square mode, size and anchors (end of upstream `setOptions`).
fn layout(
    mut bbox: BBox,
    md: &Metadata,
    options: &SymbolOptions,
    config: &RendererConfig,
) -> Layout {
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
        let hq = st
            .hq_staff_length_override()
            .unwrap_or(config.hq_staff_length);
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
    Layout {
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
    }
}

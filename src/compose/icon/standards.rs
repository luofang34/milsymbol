//! Edition-specific code meanings and native geometry from the pinned documents.

use super::{
    IconDest, Parts, SymbolState, finish_slow, number_entry, scale_for_modifiers, undefined_icon,
};
use crate::catalog::standards::{CodeKind, Scope, entry};
use crate::generated::standards::GRAPHICS;
use crate::ir::{Node, Paint, PathData, PathNode, Style};
use crate::registry::IconKey;
use crate::{BBox, RenderError, RenderMode};
use alloc::vec::Vec;

pub(crate) struct GraphicPath {
    pub d: &'static str,
    pub fill: Option<&'static str>,
    pub stroke: Option<&'static str>,
    pub width: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    Fixed,
    Main,
    FullFrame,
}

pub(crate) struct Graphic {
    pub layout: Layout,
    pub paths: &'static [GraphicPath],
    pub bbox: [f64; 4],
    pub source: crate::catalog::standards::Source,
    pub input: Option<GraphicInput>,
}

pub(crate) struct GraphicInput {
    pub field: &'static str,
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub bold: bool,
    pub subscript_decimal: bool,
    pub color: &'static str,
}

fn paint(s: &SymbolState<'_>, color: Option<&'static str>) -> Option<Paint> {
    Some(match color {
        None => Paint::None,
        Some("icon") => return s.color_of(&s.colors.icon_color),
        Some(color) => match s.options.style.mono_color.as_ref() {
            Some(_) if matches!(color, "#ffff00" | "#ffffff" | "white") => Paint::None,
            Some(mono) => Paint::Color(mono.to_str()),
            None => Paint::color(color),
        },
    })
}

fn graphic(s: &SymbolState<'_>, index: usize) -> Option<Node> {
    Some(graphic_nodes(s, GRAPHICS.get(index)?.paths))
}

pub(crate) fn context(s: &SymbolState<'_>, restricted: bool) -> Node {
    graphic_nodes(
        s,
        if restricted {
            crate::generated::standards::RESTRICTED
        } else {
            crate::generated::standards::NO_STRIKE
        },
    )
}

fn graphic_nodes(s: &SymbolState<'_>, paths: &[GraphicPath]) -> Node {
    Node::Group(
        paths
            .iter()
            .map(|p| {
                Node::Path(PathNode {
                    d: PathData::new(p.d),
                    style: Style {
                        fill: paint(s, p.fill),
                        stroke: paint(s, p.stroke),
                        stroke_width: p
                            .stroke
                            .map(|_| (p.width * s.options.style.stroke_width / 4.0).into()),
                        ..Style::default()
                    },
                })
            })
            .collect(),
    )
}

pub(super) fn version(s: &SymbolState<'_>, ss: &str) -> Option<u8> {
    if s.config.mode != RenderMode::Standards {
        return None;
    }
    match s.sidc.get(..2)? {
        "15" => Some(15),
        "16" => Some(16),
        "13" | "14" if ss == "64" => Some(16),
        _ => None,
    }
}

fn lookup(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    version: u8,
    ss: &str,
    kind: CodeKind,
    code: &str,
) -> Option<(Node, Option<usize>)> {
    let key = match kind {
        CodeKind::Entity => IconKey::Entity {
            symbol_set: ss,
            entity: code,
        },
        CodeKind::Modifier1 => IconKey::Modifier1 {
            symbol_set: ss,
            code: code.strip_prefix('0').unwrap_or(code),
        },
        CodeKind::Modifier2 => IconKey::Modifier2 {
            symbol_set: ss,
            code: code.strip_prefix('0').unwrap_or(code),
        },
    };
    if let Some(node) = parts.user_icon(key) {
        return Some((node, None));
    }
    if matches!(code, "000000" | "000") {
        return Some((Node::Group(Vec::new()), None));
    }
    let record = entry(version, ss, kind, code)?;
    match record.scope {
        Scope::Hierarchy | Scope::Empty => return Some((Node::Group(Vec::new()), None)),
        Scope::Multipoint => return None,
        Scope::Point => {}
    }
    if let Some(index) = record.graphic {
        if kind == CodeKind::Entity && GRAPHICS.get(index)?.layout == Layout::FullFrame {
            // MIL Table E-III, p. 208: the combined-combat full-frame cross
            // follows the affiliation frame, as the infantry cross does.
            let cross = number_entry(parts, ss, 0, "121100", true)?;
            let dot = graphic_nodes(s, GRAPHICS.get(index)?.paths.get(2..)?);
            return Some((Node::Group(alloc::vec![cross, dot]), None));
        }
        return Some((graphic(s, index)?, Some(index)));
    }
    let (kind, key) = match kind {
        CodeKind::Entity => (0, code),
        CodeKind::Modifier1 => (1, code.strip_prefix('0').unwrap_or(code)),
        CodeKind::Modifier2 => (2, code.strip_prefix('0').unwrap_or(code)),
    };
    let ss = if ss == "64" && kind != 0 { "60" } else { ss };
    super::defined(number_entry(parts, ss, kind, key, true)).map(|node| (node, None))
}

pub(super) fn draw(
    s: &SymbolState<'_>,
    parts: &Parts<'_>,
    ss: &str,
    version: u8,
    needs_outline: bool,
    dest: IconDest<'_>,
    bbox: BBox,
) -> Result<(), RenderError> {
    let md = s.metadata;
    let entity = md.function_id.get(..6).unwrap_or("");
    let m1 = md.flags.modifier1.as_deref().unwrap_or("000");
    let m2 = md.flags.modifier2.as_deref().unwrap_or("000");
    let mut bbox = entity_bounds(parts, version, ss, entity).unwrap_or(bbox);
    let mut post = Vec::new();
    let mut invalid = false;
    for (kind, code) in [
        (CodeKind::Entity, entity),
        (CodeKind::Modifier1, m1),
        (CodeKind::Modifier2, m2),
    ] {
        if kind != CodeKind::Entity
            && let Some(graphic) = entry(version, ss, kind, code)
                .and_then(|e| e.graphic)
                .and_then(|i| GRAPHICS.get(i))
        {
            bbox.merge(graphic_bounds(graphic).into());
        }
        match lookup(s, parts, version, ss, kind, code) {
            Some((node, Some(index))) if kind == CodeKind::Entity => {
                let (node, bounds) = layout(node, index, m1, m2);
                if let Some(bounds) = bounds {
                    bbox = bounds;
                }
                post.push(node);
            }
            Some((node, _)) if kind == CodeKind::Entity => {
                post.push(scale_for_modifiers(ss, entity, m1, m2, node)?)
            }
            Some((node, _)) => post.push(node),
            None => {
                invalid = true;
                post.push(undefined_icon(s));
            }
        }
    }
    finish_slow(s, needs_outline, post, bbox, invalid, dest)
}

fn layout(node: Node, index: usize, m1: &str, m2: &str) -> (Node, Option<BBox>) {
    let Some(graphic) = GRAPHICS.get(index).filter(|g| g.layout == Layout::Main) else {
        return (node, None);
    };
    let b = graphic_bounds(graphic);
    // APP-6 1.3.2, Figures 1-29/30: the main sector occupies 0.4L;
    // it may use an unoccupied modifier sector for legibility.
    let (height, center) = match (m1 != "000", m2 != "000") {
        (true, true) => (40.0, 100.0),
        (true, false) => (70.0, 115.0),
        (false, true) => (70.0, 85.0),
        (false, false) => (100.0, 100.0),
    };
    let half_width = (b.x1 - 100.0).abs().max((b.x2 - 100.0).abs());
    let half_height = (b.y1 - 100.0).abs().max((b.y2 - 100.0).abs());
    let factor = 1.0_f64
        .min(50.0 / half_width)
        .min(height / (2.0 * half_height))
        .min(
            (50.0 * core::f64::consts::SQRT_2 - (center - 100.0_f64).abs())
                / (half_width + half_height),
        );
    let node = Node::translate(
        100.0,
        center,
        alloc::vec![Node::scale(
            factor,
            alloc::vec![Node::translate(-100.0, -100.0, alloc::vec![node])],
        )],
    );
    (
        node,
        Some(BBox {
            x1: 100.0 + (b.x1 - 100.0) * factor,
            x2: 100.0 + (b.x2 - 100.0) * factor,
            y1: center + (b.y1 - 100.0) * factor,
            y2: center + (b.y2 - 100.0) * factor,
        }),
    )
}

fn graphic_bounds(graphic: &Graphic) -> BBox {
    let [x1, y1, x2, y2] = graphic.bbox;
    BBox { x1, y1, x2, y2 }
}

fn entity_bounds(parts: &Parts<'_>, version: u8, ss: &str, code: &str) -> Option<BBox> {
    parts
        .user_bbox(IconKey::Entity {
            symbol_set: ss,
            entity: code,
        })
        .or_else(|| {
            entry(version, ss, CodeKind::Entity, code)
                .and_then(|e| e.graphic)
                .and_then(|i| GRAPHICS.get(i))
                .map(graphic_bounds)
        })
        .or_else(|| {
            let table = super::number_table(ss)?;
            let id = crate::template::find(table.get(3)?, code)?;
            parts.resolver.bbox(id).map(super::bbox_from)
        })
}

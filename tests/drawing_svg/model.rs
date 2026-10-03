//! The SVG as an independent reader sees it: usvg resolves the cascade,
//! transforms, clip paths and paint of `Symbol::to_svg`, and the visible
//! paths are flattened into the item shape the drawing view uses.

use super::skia::{Rgba, StrokeStyle};

/// One visible path of the SVG, in pixel space.
#[derive(Debug, Clone)]
pub(crate) struct SvgItem {
    pub(crate) path: tiny_skia::Path,
    pub(crate) transform: tiny_skia::Transform,
    pub(crate) fill: Option<Rgba>,
    pub(crate) stroke: Option<StrokeStyle>,
    /// Clip regions in pixel space, outermost first.
    pub(crate) clips: Vec<tiny_skia::Path>,
}

/// The parsed document.
#[derive(Debug)]
pub(crate) struct SvgModel {
    pub(crate) tree: usvg::Tree,
    pub(crate) items: Vec<SvgItem>,
}

fn rgba(paint: &usvg::Paint, opacity: usvg::Opacity) -> Result<Rgba, String> {
    match paint {
        usvg::Paint::Color(c) => Ok(Rgba {
            rgb: [c.red, c.green, c.blue],
            opacity: opacity.get(),
        }),
        other => Err(format!("unexpected paint server {other:?}")),
    }
}

fn stroke(s: &usvg::Stroke) -> Result<StrokeStyle, String> {
    Ok(StrokeStyle {
        color: rgba(s.paint(), s.opacity())?,
        width: s.width().get(),
        dashes: s.dasharray().map(<[f32]>::to_vec).unwrap_or_default(),
        cap: match s.linecap() {
            usvg::LineCap::Butt => tiny_skia::LineCap::Butt,
            usvg::LineCap::Round => tiny_skia::LineCap::Round,
            usvg::LineCap::Square => tiny_skia::LineCap::Square,
        },
        join: match s.linejoin() {
            usvg::LineJoin::Round => tiny_skia::LineJoin::Round,
            usvg::LineJoin::Miter => tiny_skia::LineJoin::Miter,
            other => return Err(format!("unexpected line join {other:?}")),
        },
    })
}

/// Every path of a clip path's content, mapped to pixel space. Clip content
/// is in the user space of the element that references the clip path.
fn clip_paths(group: &usvg::Group, user: tiny_skia::Transform, out: &mut Vec<tiny_skia::Path>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => clip_paths(g, user, out),
            usvg::Node::Path(p) => {
                if let Some(path) = p
                    .data()
                    .clone()
                    .transform(user.pre_concat(p.abs_transform()))
                {
                    out.push(path);
                }
            }
            _ => {}
        }
    }
}

/// One clip region per `clip-path` reference; a region drawn as several
/// paths is their union, which no symbol uses, so it is an error.
fn clip_region(
    clip: &usvg::ClipPath,
    user: tiny_skia::Transform,
) -> Result<tiny_skia::Path, String> {
    if clip.clip_path().is_some() {
        return Err("nested clip paths".into());
    }
    let mut paths = Vec::new();
    clip_paths(clip.root(), user.pre_concat(clip.transform()), &mut paths);
    match <[tiny_skia::Path; 1]>::try_from(paths) {
        Ok([path]) => Ok(path),
        Err(paths) => Err(format!("clip path with {} paths", paths.len())),
    }
}

fn walk(
    group: &usvg::Group,
    clips: &[tiny_skia::Path],
    out: &mut Vec<SvgItem>,
) -> Result<(), String> {
    let mut clips = clips.to_vec();
    if let Some(clip) = group.clip_path() {
        clips.push(clip_region(clip, group.abs_transform())?);
    }
    for node in group.children() {
        match node {
            usvg::Node::Group(g) => walk(g, &clips, out)?,
            usvg::Node::Path(p) if p.is_visible() => out.push(SvgItem {
                path: p.data().clone(),
                transform: p.abs_transform(),
                fill: p.fill().map(|f| rgba(f.paint(), f.opacity())).transpose()?,
                stroke: p.stroke().map(stroke).transpose()?,
                clips: clips.clone(),
            }),
            usvg::Node::Path(_) => {}
            other => return Err(format!("unexpected SVG node {other:?}")),
        }
    }
    Ok(())
}

/// Parses `svg` and lists its visible paths in drawing order. usvg is built
/// without text support, so text elements are left to the text check.
pub(crate) fn parse(svg: &str) -> Result<SvgModel, String> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| format!("usvg cannot read the SVG: {e}"))?;
    let mut items = Vec::new();
    walk(tree.root(), &[], &mut items)?;
    Ok(SvgModel { tree, items })
}

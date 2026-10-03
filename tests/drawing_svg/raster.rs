//! Pixels: the drawing view painted by the tiny-skia consumer against resvg
//! painting the SVG. Text is left out on both sides (usvg is built without
//! fonts), so this checks that the drawing view alone is enough to paint
//! every non-text primitive: geometry, transforms, paint, opacity, strokes,
//! dashes, caps, joins and clips.

use super::skia;
use milsymbol::drawing::{ClipRegion, DrawItem, Drawing, Shape};
use tiny_skia::{FillRule, Mask, Pixmap};

/// How two renderings differ.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct Difference {
    pub(crate) pixels: usize,
    /// Pixels whose largest channel difference exceeds the threshold.
    pub(crate) differing: usize,
    pub(crate) max_channel: u8,
}

/// A channel difference that rounding in premultiplied 8-bit blending can
/// produce for identical coverage.
const CHANNEL_THRESHOLD: u8 = 2;

fn pixmap(d: &Drawing) -> Result<Pixmap, String> {
    let (w, h) = (d.size.width.ceil(), d.size.height.ceil());
    Pixmap::new(w as u32, h as u32).ok_or_else(|| format!("no pixmap of {w}x{h}"))
}

/// The intersection of `clips`, in pixels.
fn clip_mask(d: &Drawing, clips: &[ClipRegion], w: u32, h: u32) -> Result<Mask, String> {
    let to_pixels = skia::transform(&skia::viewport(d));
    let mut mask = Mask::new(w, h).ok_or("no clip mask")?;
    for (i, clip) in clips.iter().enumerate() {
        let path = skia::path(&clip.segments).ok_or("clip region without geometry")?;
        if i == 0 {
            mask.fill_path(&path, FillRule::Winding, true, to_pixels);
        } else {
            mask.intersect_path(&path, FillRule::Winding, true, to_pixels);
        }
    }
    Ok(mask)
}

/// An 8-bit colour paint, as rasterizers are usually handed one.
fn solid(c: skia::Rgba) -> tiny_skia::Paint<'static> {
    let [r, g, b] = c.rgb;
    let alpha = (c.opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    let mut paint = tiny_skia::Paint::default();
    paint.set_color_rgba8(r, g, b, alpha);
    paint.anti_alias = true;
    paint
}

fn paint_item(d: &Drawing, item: &DrawItem, out: &mut Pixmap) -> Result<(), String> {
    let Some(path) = skia::shape(&item.shape) else {
        return Ok(());
    };
    let transform = skia::transform(&skia::viewport(d).then(item.transform));
    if let Some(fill) = skia::fill(&item.appearance)? {
        out.fill_path(&path, &solid(fill), FillRule::Winding, transform, None);
    }
    if let Some(s) = skia::stroke(&item.appearance)? {
        let stroke = tiny_skia::Stroke {
            width: s.width,
            line_cap: s.cap,
            line_join: s.join,
            dash: tiny_skia::StrokeDash::new(s.dashes.clone(), 0.0),
            ..tiny_skia::Stroke::default()
        };
        out.stroke_path(&path, &solid(s.color), &stroke, transform, None);
    }
    Ok(())
}

/// Paints the drawing view's non-text items. A clip region applies to the
/// group it came from as a whole (SVG paints the group, then clips it), so
/// each run of items sharing the same clips is painted into a layer that is
/// masked once.
pub(crate) fn drawing(d: &Drawing) -> Result<Pixmap, String> {
    let mut out = pixmap(d)?;
    let items: Vec<&DrawItem> = d
        .items
        .iter()
        .filter(|i| !matches!(i.shape, Shape::Text(_)))
        .collect();
    for run in items.chunk_by(|a, b| a.clips == b.clips) {
        let Some(first) = run.first() else { continue };
        if first.clips.is_empty() {
            for item in run {
                paint_item(d, item, &mut out)?;
            }
            continue;
        }
        let mut layer = pixmap(d)?;
        for item in run {
            paint_item(d, item, &mut layer)?;
        }
        layer.apply_mask(&clip_mask(d, &first.clips, out.width(), out.height())?);
        let blend = tiny_skia::PixmapPaint::default();
        out.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &blend,
            tiny_skia::Transform::identity(),
            None,
        );
    }
    Ok(out)
}

/// Paints the SVG with resvg at the drawing's pixel size.
pub(crate) fn svg(tree: &usvg::Tree, d: &Drawing) -> Result<Pixmap, String> {
    let mut out = pixmap(d)?;
    resvg::render(tree, tiny_skia::Transform::identity(), &mut out.as_mut());
    Ok(out)
}

/// Compares two renderings of the same size, premultiplied RGBA.
pub(crate) fn difference(a: &Pixmap, b: &Pixmap) -> Result<Difference, String> {
    if (a.width(), a.height()) != (b.width(), b.height()) {
        return Err("renderings differ in size".into());
    }
    let mut diff = Difference::default();
    for (p, q) in a.data().chunks_exact(4).zip(b.data().chunks_exact(4)) {
        let channel = p
            .iter()
            .zip(q)
            .map(|(x, y)| x.abs_diff(*y))
            .max()
            .unwrap_or(0);
        diff.pixels += 1;
        diff.max_channel = diff.max_channel.max(channel);
        diff.differing += usize::from(channel > CHANNEL_THRESHOLD);
    }
    Ok(diff)
}

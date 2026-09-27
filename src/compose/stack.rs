//! Stacked frames behind the symbol (upstream `stack-extension.js`).

use super::{PartOutput, SymbolState};
use crate::ir::Node;
use alloc::vec::Vec;

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, crate::RenderError> {
    let base = s.metadata.geometry_bbox();
    let Some(count) = s.options.stack else {
        return Ok(PartOutput::new(Vec::new(), Vec::new(), base));
    };
    let Some((pre, post)) = super::base_geometry::frame(s)? else {
        return Ok(PartOutput::new(Vec::new(), Vec::new(), base));
    };
    let mut outlines = Vec::new();
    let mut frames = Vec::new();
    let mut i = count;
    while i >= 1.0 {
        outlines.push(Node::translate(15.0 * i, 9.0 * i, pre.clone()));
        frames.push(Node::translate(15.0 * i, 9.0 * i, post.clone()));
        i -= 1.0;
    }
    let mut bbox = base;
    bbox.x2 += 15.0 * count;
    bbox.y2 += 10.0 * count;
    Ok(PartOutput::new(
        alloc::vec![Node::Group(outlines)],
        alloc::vec![Node::Group(frames)],
        bbox,
    ))
}

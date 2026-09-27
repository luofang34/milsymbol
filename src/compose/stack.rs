//! Stacked frames behind the symbol (upstream `stack-extension.js`).

use super::{PartOutput, SymbolState};
use crate::ir::Node;
use alloc::vec::Vec;

/// Largest `stack` rendered. Upstream loops `for (i = stack; i >= 1; i--)`,
/// which never terminates for non-finite or very large values (`i - 1 == i`).
pub(crate) const MAX_STACK: f64 = 1000.0;

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, crate::RenderError> {
    let base = s.metadata.geometry_bbox();
    let Some(count) = s.options.stack else {
        return Ok(PartOutput::new(Vec::new(), Vec::new(), base));
    };
    if !count.is_finite() || count > MAX_STACK {
        return Err(crate::RenderError::InvalidOption {
            name: "stack",
            reason: "must be finite and at most 1000",
        });
    }
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

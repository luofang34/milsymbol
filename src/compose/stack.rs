//! Stacked frames behind the symbol (upstream `stack-extension.js`).

use super::{PartOutput, SymbolState};
use crate::bbox::BBox;
use crate::ir::Node;
use crate::node_list::NodeAcc;
use alloc::vec::Vec;

/// Largest `stack` rendered. Upstream loops `for (i = stack; i >= 1; i--)`,
/// which never terminates for non-finite or very large values (`i - 1 == i`).
pub(crate) const MAX_STACK: f64 = 1000.0;

pub(super) fn draw(s: &SymbolState<'_>) -> Result<PartOutput, crate::RenderError> {
    let Some(count) = s.options.stack else {
        return Ok(PartOutput::new(
            Vec::new(),
            Vec::new(),
            s.metadata.geometry_bbox(),
        ));
    };
    if !count.is_finite() || count > MAX_STACK {
        return Err(crate::RenderError::InvalidOption {
            name: "stack",
            reason: "must be finite and at most 1000",
        });
    }
    let (mut pre_one, mut post_acc, mut base) = (None, NodeAcc::default(), BBox::default());
    if !super::base_geometry::frame_into(s, &mut pre_one, &mut post_acc, &mut base)? {
        return Ok(PartOutput::new(Vec::new(), Vec::new(), base));
    }
    let pre = pre_one.map_or_else(Vec::new, |o| alloc::vec![o]);
    let post = post_acc.into_vec();
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

//! Clip-path ids: generated ids never repeat or collide with requested
//! ones, and repeated requested ids are made unique.

use super::sanitize::sanitize_id;
use crate::ir::Node;
use alloc::collections::BTreeSet;
use alloc::string::String;
use core::fmt::Write as _;

pub(super) struct ClipIds<'a> {
    prefix: &'a str,
    nodes: &'a [Node],
    counter: u32,
    /// Sanitized ids requested anywhere in the document; built on first use.
    requested: Option<BTreeSet<String>>,
    used: BTreeSet<String>,
}

impl<'a> ClipIds<'a> {
    pub(super) fn new(prefix: &'a str, nodes: &'a [Node]) -> Self {
        ClipIds {
            prefix,
            nodes,
            counter: 0,
            requested: None,
            used: BTreeSet::new(),
        }
    }

    /// `{prefix}clip-{kind}-{n}` for the next free `n`.
    pub(super) fn generate(&mut self, kind: &str) -> String {
        let nodes = self.nodes;
        let requested = self.requested.get_or_insert_with(|| {
            let mut ids = BTreeSet::new();
            collect_requested(nodes, &mut ids);
            ids
        });
        loop {
            let mut id = String::from(self.prefix);
            write!(id, "clip-{kind}-{}", self.counter).ok();
            self.counter = self.counter.wrapping_add(1);
            let bare = id.get(self.prefix.len()..).unwrap_or("");
            if !requested.contains(bare) && !self.used.contains(&id) {
                self.used.insert(id.clone());
                return id;
            }
        }
    }

    /// The id for a requested `clip_id`: sanitized, prefixed, and suffixed
    /// with `-1`, `-2`, … if already used. `None` asks for a generated id.
    pub(super) fn request(&mut self, requested: &str) -> Option<String> {
        let mut base = String::from(self.prefix);
        base.push_str(&sanitize_id(requested)?);
        let mut id = base.clone();
        let mut n = 0u32;
        while self.used.contains(&id) {
            n = n.wrapping_add(1);
            id.clone_from(&base);
            write!(id, "-{n}").ok();
        }
        self.used.insert(id.clone());
        Some(id)
    }
}

fn collect_requested(nodes: &[Node], ids: &mut BTreeSet<String>) {
    for n in nodes {
        if let Node::Clip(c) = n {
            if let Some(id) = c.clip_id.as_deref().and_then(sanitize_id) {
                ids.insert(id);
            }
        }
        collect_requested(n.children().unwrap_or(&[]), ids);
    }
}

/// Keeps the characters valid in an XML id.
pub(super) fn sanitize_prefix(prefix: &str) -> String {
    prefix
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | ':' | '-'))
        .collect()
}

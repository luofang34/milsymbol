//! Inline-first instruction list (`0..=3` [`Node`]s on the stack, `Vec<Node>`
//! for larger symbols), with an in-place [`NodeAcc`] builder that avoids moving
//! earlier nodes while composing.

use crate::ir::Node;
use alloc::vec::Vec;
use core::ops::Deref;

/// Holds up to three [`Node`]s inline without heap allocation, spilling to
/// `Vec<Node>` only when a symbol produces four or more top-level instructions.
#[derive(Debug, Clone, Default)]
#[allow(clippy::large_enum_variant)]
pub(crate) enum NodeList {
    #[default]
    Empty,
    One([Node; 1]),
    Two([Node; 2]),
    Three([Node; 3]),
    Heap(Vec<Node>),
}

impl NodeList {
    pub(crate) fn as_slice(&self) -> &[Node] {
        match self {
            NodeList::Empty => &[],
            NodeList::One(a) => a.as_slice(),
            NodeList::Two(a) => a.as_slice(),
            NodeList::Three(a) => a.as_slice(),
            NodeList::Heap(v) => v.as_slice(),
        }
    }
}

impl Deref for NodeList {
    type Target = [Node];

    fn deref(&self) -> &[Node] {
        self.as_slice()
    }
}

/// In-place instruction accumulator used during composition so each `push`
/// writes only the target slot (or `Vec::push` once spilled) without moving
/// already-written [`Node`]s.
#[derive(Default)]
pub(crate) struct NodeAcc {
    spilled: bool,
    len_inline: u8,
    first: Option<Node>,
    second: Option<Node>,
    third: Option<Node>,
    heap: Vec<Node>,
}

impl NodeAcc {
    pub(crate) fn len(&self) -> usize {
        if self.spilled {
            self.heap.len()
        } else {
            usize::from(self.len_inline)
        }
    }

    pub(crate) fn push(&mut self, n: Node) {
        if self.spilled {
            self.heap.push(n);
            return;
        }
        match self.len_inline {
            0 => {
                self.first = Some(n);
                self.len_inline = 1;
            }
            1 => {
                self.second = Some(n);
                self.len_inline = 2;
            }
            2 => {
                self.third = Some(n);
                self.len_inline = 3;
            }
            _ => {
                self.spill_with_cap(8);
                self.heap.push(n);
            }
        }
    }

    fn spill_with_cap(&mut self, cap: usize) {
        if !self.spilled {
            self.heap.reserve(cap);
            if let Some(a) = self.first.take() {
                self.heap.push(a);
            }
            if let Some(b) = self.second.take() {
                self.heap.push(b);
            }
            if let Some(c) = self.third.take() {
                self.heap.push(c);
            }
            self.len_inline = 0;
            self.spilled = true;
        }
    }

    pub(crate) fn extend_vec(&mut self, v: Vec<Node>) {
        if v.is_empty() {
            return;
        }
        if !self.spilled && usize::from(self.len_inline) + v.len() <= 3 {
            for n in v {
                self.push(n);
            }
        } else {
            let cap = usize::from(self.len_inline) + v.len();
            self.spill_with_cap(cap.max(8));
            self.heap.extend(v);
        }
    }

    pub(crate) fn prepend_one(&mut self, n: Node) {
        if self.spilled {
            self.heap.insert(0, n);
            return;
        }
        match self.len_inline {
            0 => {
                self.first = Some(n);
                self.len_inline = 1;
            }
            1 => {
                self.second = self.first.replace(n);
                self.len_inline = 2;
            }
            2 => {
                self.third = self.second.take();
                self.second = self.first.replace(n);
                self.len_inline = 3;
            }
            _ => {
                let mut v = Vec::with_capacity(8);
                v.push(n);
                if let Some(a) = self.first.take() {
                    v.push(a);
                }
                if let Some(b) = self.second.take() {
                    v.push(b);
                }
                if let Some(c) = self.third.take() {
                    v.push(c);
                }
                self.len_inline = 0;
                self.heap = v;
                self.spilled = true;
            }
        }
    }

    pub(crate) fn prepend_vec(&mut self, mut first_vec: Vec<Node>) {
        if first_vec.is_empty() {
            return;
        }
        if !self.spilled && self.len_inline == 0 && first_vec.len() <= 3 {
            for n in first_vec {
                self.push(n);
            }
            return;
        }
        if self.spilled {
            first_vec.append(&mut self.heap);
        } else {
            first_vec.reserve(usize::from(self.len_inline));
            if let Some(a) = self.first.take() {
                first_vec.push(a);
            }
            if let Some(b) = self.second.take() {
                first_vec.push(b);
            }
            if let Some(c) = self.third.take() {
                first_vec.push(c);
            }
            self.len_inline = 0;
            self.spilled = true;
        }
        self.heap = first_vec;
    }

    pub(crate) fn last_mut(&mut self) -> Option<&mut Node> {
        if self.spilled {
            self.heap.last_mut()
        } else {
            match self.len_inline {
                3 => self.third.as_mut(),
                2 => self.second.as_mut(),
                1 => self.first.as_mut(),
                _ => None,
            }
        }
    }

    pub(crate) fn contains_missing(&self) -> bool {
        if self.spilled {
            crate::ir::contains_missing(&self.heap)
        } else {
            let check = |s: &Option<Node>| {
                s.as_ref()
                    .is_some_and(|n| crate::ir::contains_missing(core::slice::from_ref(n)))
            };
            check(&self.first) || check(&self.second) || check(&self.third)
        }
    }

    pub(crate) fn into_vec(mut self) -> Vec<Node> {
        if self.spilled {
            return self.heap;
        }
        let mut v = Vec::with_capacity(usize::from(self.len_inline));
        if let Some(a) = self.first.take() {
            v.push(a);
        }
        if let Some(b) = self.second.take() {
            v.push(b);
        }
        if let Some(c) = self.third.take() {
            v.push(c);
        }
        v
    }

    pub(crate) fn finish(self) -> NodeList {
        if self.spilled {
            return NodeList::Heap(self.heap);
        }
        match (self.first, self.second, self.third) {
            (None, _, _) => NodeList::Empty,
            (Some(a), None, _) => NodeList::One([a]),
            (Some(a), Some(b), None) => NodeList::Two([a, b]),
            (Some(a), Some(b), Some(c)) => NodeList::Three([a, b, c]),
        }
    }
}

//! Lazy canonical values: only the current object's fields are buffered.

use super::{Json, fields, write_str};
use crate::color::{ColorMode, ColorSet};
use crate::geometry::{BaseGeometry, GeomShape};
use crate::ir::{Node, Point};
use crate::{BBox, Size, Symbol};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy)]
pub(crate) enum Value<'a> {
    Null,
    Bool(bool),
    Num(f64),
    Str(&'a str),
    Node(&'a Node),
    Instructions(&'a [Node]),
    ColorMode(&'a ColorMode),
    Colors(&'a ColorSet),
    Metadata(&'a crate::metadata::Metadata),
    Geometry(Option<&'static BaseGeometry>),
    Shape(GeomShape),
    BBox(BBox),
    Point(Point),
    Size(Size),
    Options(&'a Symbol),
    Validity(&'a Symbol),
    Symbol(&'a Symbol),
}

type Entry<'a> = (&'a str, Value<'a>, usize);

/// Closed schemas fit on the stack. Unbounded extension option keys spill
/// to the heap instead of overflowing or truncating the canonical record.
pub(super) struct Object<'a, const N: usize> {
    stack: [Entry<'a>; N],
    len: usize,
    spill: Vec<Entry<'a>>,
}

impl<'a, const N: usize> Object<'a, N> {
    fn new() -> Self {
        Self {
            stack: [("", Value::Null, 0); N],
            len: 0,
            spill: Vec::new(),
        }
    }

    pub(super) fn put(&mut self, key: &'a str, value: Value<'a>) -> &mut Self {
        let entry = (key, value, self.len);
        if self.spill.is_empty() {
            if let Some(slot) = self.stack.get_mut(self.len) {
                *slot = entry;
                self.len += 1;
                return self;
            }
            self.spill.extend_from_slice(&self.stack);
        }
        self.spill.push(entry);
        self.len += 1;
        self
    }

    pub(super) fn opt(&mut self, key: &'a str, value: Option<Value<'a>>) -> &mut Self {
        if let Some(value) = value {
            self.put(key, value);
        }
        self
    }

    /// Extra fields have unique keys; only the native schema can collide.
    pub(super) fn extend_missing(&mut self, extra: impl IntoIterator<Item = (&'a str, Value<'a>)>) {
        let native_len = self.len;
        for (key, value) in extra {
            if !self
                .fields()
                .iter()
                .take(native_len)
                .any(|(k, _, _)| *k == key)
            {
                self.put(key, value);
            }
        }
    }

    fn fields(&mut self) -> &mut [Entry<'a>] {
        if self.spill.is_empty() {
            self.stack.get_mut(..self.len).unwrap_or_default()
        } else {
            &mut self.spill
        }
    }
}

fn object<'a, R, const N: usize>(
    build: impl FnOnce(&mut Object<'a, N>),
    visit: impl FnOnce(&mut [Entry<'a>]) -> R,
) -> R {
    let mut obj = Object::new();
    build(&mut obj);
    visit(obj.fields())
}

impl<'a> Value<'a> {
    pub(crate) fn write(self, out: &mut String) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if b { "true" } else { "false" }),
            Self::Num(n) if n.is_finite() => crate::js::write_number(out, n),
            Self::Num(_) => out.push_str("null"),
            Self::Str(s) => write_str(out, s),
            Self::Instructions(nodes) => {
                out.push('[');
                for (i, n) in nodes.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    fields::node_value(n).write(out);
                }
                out.push(']');
            }
            _ => self.with_object(|fields| {
                // Position breaks ties, preserving duplicate-key order without
                // the scratch allocation of a stable sort.
                fields.sort_unstable_by(|a, b| super::compare_keys(a.0, b.0).then(a.2.cmp(&b.2)));
                out.push('{');
                for (i, (key, value, _)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, key);
                    out.push(':');
                    value.write(out);
                }
                out.push('}');
            }),
        }
    }

    pub(crate) fn to_json(self) -> Json {
        match self {
            Self::Null => Json::Null,
            Self::Bool(b) => Json::Bool(b),
            Self::Num(n) => Json::Num(n),
            Self::Str(s) => Json::Str(String::from(s)),
            Self::Instructions(nodes) => Json::Arr(
                nodes
                    .iter()
                    .map(|n| fields::node_value(n).to_json())
                    .collect(),
            ),
            _ => self.with_object(|fields| {
                Json::Obj(
                    fields
                        .iter()
                        .map(|(k, v, _)| (String::from(*k), v.to_json()))
                        .collect(),
                )
            }),
        }
    }

    fn with_object<R>(self, visit: impl FnOnce(&mut [Entry<'a>]) -> R) -> R {
        match self {
            Self::Node(n) => object(|o| fields::node(n, o), visit),
            Self::ColorMode(m) => object(|o| fields::color_mode(m, o), visit),
            Self::Colors(c) => object(|o| fields::colors(c, o), visit),
            Self::Metadata(md) => object(|o| fields::metadata(md, o), visit),
            Self::Options(s) => object(|o| fields::options(s.sidc(), s.options(), o), visit),
            Self::Symbol(s) => object(|o| fields::symbol(s, o), visit),
            Self::Validity(s) => object(|o| fields::validity(s, o), visit),
            _ => self.geometry_object(visit),
        }
    }

    fn geometry_object<R>(self, visit: impl FnOnce(&mut [Entry<'a>]) -> R) -> R {
        object::<_, 4>(
            |o| match self {
                Self::BBox(b) => {
                    o.put("x1", Self::Num(b.x1))
                        .put("y1", Self::Num(b.y1))
                        .put("x2", Self::Num(b.x2))
                        .put("y2", Self::Num(b.y2));
                }
                Self::Point(p) => {
                    o.put("x", Self::Num(p.x)).put("y", Self::Num(p.y));
                }
                Self::Size(s) => {
                    o.put("width", Self::Num(s.width))
                        .put("height", Self::Num(s.height));
                }
                Self::Geometry(g) => {
                    o.put("g", g.map_or(Self::Str(""), |g| Self::Shape(g.shape)))
                        .put(
                            "bbox",
                            Self::BBox(g.map_or_else(BBox::default, BaseGeometry::bbox)),
                        );
                }
                Self::Shape(GeomShape::Path(d)) => {
                    o.put("type", Self::Str("path")).put("d", Self::Str(d));
                }
                Self::Shape(GeomShape::Circle { cx, cy, r }) => {
                    o.put("type", Self::Str("circle"))
                        .put("cx", Self::Num(cx))
                        .put("cy", Self::Num(cy))
                        .put("r", Self::Num(r));
                }
                _ => {}
            },
            visit,
        )
    }
}

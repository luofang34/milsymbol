//! Native drawing IR.
//!
//! A rendered symbol is a tree of [`Node`]s mirroring upstream's semantic
//! draw instructions (`path`, `circle`, `text`, `translate`, `rotate`,
//! `scale`). Renderers — the bundled SVG serializer or a GPU backend — walk
//! this tree; nothing in it is an SVG string except the path-data source
//! kept alongside [`PathData::segments`] for byte-exact SVG output.
//!
//! Upstream stores attributes as loosely typed JavaScript values; the IR keeps
//! the distinctions that change output: an absent attribute (`None`), an
//! explicit `false` paint ([`Paint::None`]), and numbers that upstream happens
//! to store as strings or booleans ([`Num`]).

use alloc::borrow::Cow;
use alloc::vec::Vec;

mod path;
pub use path::{PathData, PathParseError, Point, Segment};

/// Shorthand for the string type used throughout the IR.
pub type Str = Cow<'static, str>;

/// A paint (fill or stroke) value.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// Explicitly no paint (upstream `false`, SVG `none`).
    None,
    /// A CSS colour string, passed through as given.
    Color(Str),
}

impl Paint {
    /// Creates a colour paint.
    pub fn color(c: impl Into<Str>) -> Self {
        Paint::Color(c.into())
    }

    /// The colour string, if any.
    pub fn as_color(&self) -> Option<&str> {
        match self {
            Paint::None => None,
            Paint::Color(c) => Some(c),
        }
    }
}

/// A numeric attribute with JavaScript's loose typing preserved.
///
/// Almost every value is [`Num::Number`]; a handful of upstream icons store
/// numbers as strings (`fontsize: "45"`) or booleans (`strokewidth: false`).
#[derive(Debug, Clone, PartialEq)]
pub enum Num {
    /// A number.
    Number(f64),
    /// A numeric string.
    Text(Str),
    /// A boolean used where a number is expected.
    Bool(bool),
}

impl Num {
    /// JavaScript `Number(value)`.
    pub fn value(&self) -> f64 {
        match self {
            Num::Number(n) => *n,
            Num::Text(s) => crate::js::string_to_number(s),
            Num::Bool(b) => f64::from(u8::from(*b)),
        }
    }
}

impl From<f64> for Num {
    fn from(v: f64) -> Self {
        Num::Number(v)
    }
}

/// Presentation attributes shared by every node type.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Style {
    /// Fill paint; `None` leaves the attribute unset.
    pub fill: Option<Paint>,
    /// Fill opacity.
    pub fill_opacity: Option<Num>,
    /// Stroke paint; `None` leaves the attribute unset.
    pub stroke: Option<Paint>,
    /// Stroke width in symbol units (before `non_scaling_stroke`).
    pub stroke_width: Option<Num>,
    /// Stroke dash pattern, e.g. `"4,4"`.
    pub stroke_dasharray: Option<Str>,
    /// Line cap and join style (`butt`, `round`, `square`).
    pub line_cap: Option<Str>,
    /// Multiplier applied to the stroke width so scaled icons keep their
    /// nominal stroke.
    pub non_scaling_stroke: Option<f64>,
    /// Marks fills that the `styleFill` option may override.
    pub style_fill: Option<bool>,
    /// Marks nodes that belong to an icon (informational).
    pub icon: Option<bool>,
    /// Clip path data applied to this node.
    pub clip_path: Option<Str>,
}

/// A path node.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PathNode {
    /// Path geometry.
    pub d: PathData,
    /// Presentation attributes.
    pub style: Style,
}

/// A circle node.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct CircleNode {
    /// Centre x.
    pub cx: Num,
    /// Centre y.
    pub cy: Num,
    /// Radius.
    pub r: Num,
    /// Presentation attributes.
    pub style: Style,
}

/// A text node.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TextNode {
    /// Anchor x.
    pub x: Num,
    /// Anchor y.
    pub y: Num,
    /// The text content (unescaped).
    pub text: Str,
    /// Font size.
    pub font_size: Option<Num>,
    /// Font family list.
    pub font_family: Option<Str>,
    /// Font weight.
    pub font_weight: Option<Str>,
    /// Horizontal anchoring (`start`, `middle`, `end`).
    pub text_anchor: Option<Str>,
    /// Vertical alignment (SVG `dominant-baseline`).
    pub alignment_baseline: Option<Str>,
    /// Presentation attributes.
    pub style: Style,
}

/// A group translated by `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct TranslateNode {
    /// Horizontal offset.
    pub x: Num,
    /// Vertical offset.
    pub y: Num,
    /// Children.
    pub draw: Vec<Node>,
    /// Presentation attributes inherited by children.
    pub style: Style,
}

/// A group rotated by `degree` around `(x, y)`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RotateNode {
    /// Rotation in degrees, clockwise.
    pub degree: Num,
    /// Rotation centre x.
    pub x: Num,
    /// Rotation centre y.
    pub y: Num,
    /// Children.
    pub draw: Vec<Node>,
    /// Presentation attributes inherited by children.
    pub style: Style,
}

/// A group uniformly scaled by `factor` about the origin.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ScaleNode {
    /// Scale factor.
    pub factor: Num,
    /// Children.
    pub draw: Vec<Node>,
    /// Presentation attributes inherited by children.
    pub style: Style,
}

/// A group clipped by a path (extension instruction).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClipNode {
    /// Requested clip id; sanitized on output.
    pub clip_id: Option<Str>,
    /// Clip geometry.
    pub d: PathData,
    /// Children.
    pub draw: Vec<Node>,
    /// Presentation attributes.
    pub style: Style,
}

/// One drawing instruction.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Node {
    /// A path.
    Path(PathNode),
    /// A circle.
    Circle(CircleNode),
    /// A text run.
    Text(TextNode),
    /// A translated group.
    Translate(TranslateNode),
    /// A rotated group.
    Rotate(RotateNode),
    /// A scaled group.
    Scale(ScaleNode),
    /// A clipped group (extension instruction).
    Clip(ClipNode),
    /// A trusted raw SVG fragment supplied by an extension.
    ///
    /// Built-in symbols never produce this. The SVG renderer drops fragments
    /// matching upstream's script/handler blocklist.
    TrustedSvg(Str),
    /// A nested list, kept because upstream nests instruction arrays.
    Group(Vec<Node>),
    /// A missing instruction (upstream `undefined`); marks the symbol invalid.
    Missing,
    /// A non-object value inside an instruction list; renderers ignore it.
    Scalar(Num),
    /// An instruction without a type: upstream's outline of a non-object
    /// entry. The SVG renderer emits only its attributes, as upstream does.
    Bare(Style),
}

impl Node {
    /// The node's presentation attributes, if it has any.
    pub fn style(&self) -> Option<&Style> {
        match self {
            Node::Path(n) => Some(&n.style),
            Node::Circle(n) => Some(&n.style),
            Node::Text(n) => Some(&n.style),
            Node::Translate(n) => Some(&n.style),
            Node::Rotate(n) => Some(&n.style),
            Node::Scale(n) => Some(&n.style),
            Node::Clip(n) => Some(&n.style),
            Node::Bare(s) => Some(s),
            Node::TrustedSvg(_) | Node::Group(_) | Node::Missing | Node::Scalar(_) => None,
        }
    }

    /// Mutable access to the node's presentation attributes.
    pub fn style_mut(&mut self) -> Option<&mut Style> {
        match self {
            Node::Path(n) => Some(&mut n.style),
            Node::Circle(n) => Some(&mut n.style),
            Node::Text(n) => Some(&mut n.style),
            Node::Translate(n) => Some(&mut n.style),
            Node::Rotate(n) => Some(&mut n.style),
            Node::Scale(n) => Some(&mut n.style),
            Node::Clip(n) => Some(&mut n.style),
            Node::Bare(s) => Some(s),
            Node::TrustedSvg(_) | Node::Group(_) | Node::Missing | Node::Scalar(_) => None,
        }
    }

    /// Child instructions of a group-like node.
    pub fn children(&self) -> Option<&[Node]> {
        match self {
            Node::Translate(n) => Some(&n.draw),
            Node::Rotate(n) => Some(&n.draw),
            Node::Scale(n) => Some(&n.draw),
            Node::Clip(n) => Some(&n.draw),
            Node::Group(v) => Some(v),
            _ => None,
        }
    }

    /// Mutable access to the child instructions of a group-like node.
    pub fn children_mut(&mut self) -> Option<&mut Vec<Node>> {
        match self {
            Node::Translate(n) => Some(&mut n.draw),
            Node::Rotate(n) => Some(&mut n.draw),
            Node::Scale(n) => Some(&mut n.draw),
            Node::Clip(n) => Some(&mut n.draw),
            Node::Group(v) => Some(v),
            _ => None,
        }
    }

    /// Creates a path node with default style.
    pub fn path(d: impl Into<Str>) -> Self {
        Node::Path(PathNode::new(PathData::new(d)))
    }

    /// Creates a path node from already built path data.
    pub fn path_data(d: PathData) -> Self {
        Node::Path(PathNode::new(d))
    }

    /// Creates a circle node with default style.
    pub fn circle(cx: f64, cy: f64, r: f64) -> Self {
        Node::Circle(CircleNode {
            cx: cx.into(),
            cy: cy.into(),
            r: r.into(),
            style: Style::default(),
        })
    }

    /// Creates a text node with default attributes; see [`TextNode::new`]
    /// to set fonts and anchoring first.
    pub fn text(x: f64, y: f64, text: impl Into<Str>) -> Self {
        Node::Text(TextNode::new(x, y, text))
    }

    /// Creates a translate group.
    pub fn translate(x: f64, y: f64, draw: Vec<Node>) -> Self {
        Node::Translate(TranslateNode {
            x: x.into(),
            y: y.into(),
            draw,
            style: Style::default(),
        })
    }

    /// Creates a group rotated by `degree` (clockwise) around `(x, y)`.
    pub fn rotate(degree: f64, x: f64, y: f64, draw: Vec<Node>) -> Self {
        Node::Rotate(RotateNode {
            degree: degree.into(),
            x: x.into(),
            y: y.into(),
            draw,
            style: Style::default(),
        })
    }

    /// Creates a group scaled by `factor` about the origin.
    pub fn scale(factor: f64, draw: Vec<Node>) -> Self {
        Node::Scale(ScaleNode {
            factor: factor.into(),
            draw,
            style: Style::default(),
        })
    }

    /// Creates a group clipped by `d`; `clip_id` requests an id for the
    /// clip path (sanitized, and made unique, on SVG output).
    pub fn clip(d: PathData, clip_id: Option<Str>, draw: Vec<Node>) -> Self {
        Node::Clip(ClipNode {
            clip_id,
            d,
            draw,
            style: Style::default(),
        })
    }
}

impl PathNode {
    /// A path with default style.
    pub fn new(d: PathData) -> Self {
        PathNode {
            d,
            style: Style::default(),
        }
    }
}

impl TextNode {
    /// A text run with default attributes.
    pub fn new(x: f64, y: f64, text: impl Into<Str>) -> Self {
        TextNode {
            x: x.into(),
            y: y.into(),
            text: text.into(),
            font_size: None,
            font_family: None,
            font_weight: None,
            text_anchor: None,
            alignment_baseline: None,
            style: Style::default(),
        }
    }
}

/// Parses every path in `nodes` (recursively) and caches its segments, so
/// renderers that walk the tree repeatedly do not re-parse path data.
pub fn parse_paths(nodes: &mut [Node]) -> Result<(), PathParseError> {
    for n in nodes {
        match n {
            Node::Path(p) => p.d.cache_segments()?,
            Node::Clip(c) => {
                c.d.cache_segments()?;
                parse_paths(&mut c.draw)?;
            }
            Node::Translate(t) => parse_paths(&mut t.draw)?,
            Node::Rotate(r) => parse_paths(&mut r.draw)?,
            Node::Scale(s) => parse_paths(&mut s.draw)?,
            Node::Group(g) => parse_paths(g)?,
            _ => {}
        }
    }
    Ok(())
}

/// Whether any instruction in `nodes` is [`Node::Missing`], directly or nested.
pub fn contains_missing(nodes: &[Node]) -> bool {
    nodes
        .iter()
        .any(|n| matches!(n, Node::Missing) || n.children().is_some_and(contains_missing))
}

#[cfg(test)]
mod tests;

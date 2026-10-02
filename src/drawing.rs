//! A resolved, typed view of a symbol for renderers other than SVG.
//!
//! [`Symbol::instructions`](crate::Symbol::instructions) keeps the drawing
//! tree the way milsymbol.js builds it, with nested transforms, styles that
//! cascade from groups, and values that may be numbers or strings. A
//! [`Drawing`] is the same picture flattened for a GPU, canvas or scene
//! adapter: every [`DrawItem`] carries the complete transform, paint and
//! clip it is drawn with, in plain typed fields.
//!
//! Names shared with [`ir`](crate::ir): `drawing::Paint` is the resolved paint
//! of an item (`Solid` colour or `None`), while `ir::Paint` is a node's paint
//! in the instruction tree (`Color` string passed through as given, or
//! `None`); and
//! `ir::Style` holds optional presentation attributes of a node, which have
//! nothing to do with the symbol [`options::Style`](crate::options::Style) set
//! by the caller.
//!
//! The view follows the SVG output, including what the serializer drops:
//! colours, dash arrays, line caps, text anchors, font weights and font
//! families that the SVG writer rejects are resolved the same way here.
//!
//! ```
//! use milsymbol::Renderer;
//! use milsymbol::drawing::Shape;
//!
//! let symbol = Renderer::default().symbol("10031000001211000000").render()?;
//! let drawing = symbol.drawing();
//! let paths = drawing.items.iter().filter(|i| matches!(i.shape, Shape::Path(_))).count();
//! assert!(paths > 0);
//! # Ok::<(), milsymbol::RenderError>(())
//! ```

use crate::ir::{Point, Segment};
use crate::options::Color;
use alloc::string::String;
use alloc::vec::Vec;

mod build;
mod transform;

pub use transform::Transform;

/// A symbol as a flat list of drawing items.
///
/// Items are drawn in order, later ones over earlier ones.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Drawing {
    /// The primitives, in drawing order.
    pub items: Vec<DrawItem>,
    /// The visible region in symbol units (the SVG `viewBox`).
    pub view_box: ViewBox,
    /// Size in pixels the view box is drawn at.
    pub size: crate::Size,
    /// The point, in pixels from the top-left, to place on the map position.
    pub anchor: Point,
    /// Number of raw SVG fragments from extensions that are not part of
    /// [`Drawing::items`]; they can only be drawn by an SVG renderer.
    pub omitted_raw_svg: usize,
    /// Number of paths whose data is malformed. Their valid prefix is drawn,
    /// as SVG renderers do.
    pub invalid_paths: usize,
}

/// A rectangle in symbol units.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewBox {
    /// Left.
    pub x: f64,
    /// Top.
    pub y: f64,
    /// Width.
    pub width: f64,
    /// Height.
    pub height: f64,
}

/// One primitive with everything needed to draw it.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct DrawItem {
    /// What to draw, in the coordinates that [`DrawItem::transform`] maps to
    /// symbol units.
    pub shape: Shape,
    /// How to paint it.
    pub appearance: Appearance,
    /// Maps the shape's coordinates to symbol units.
    pub transform: Transform,
    /// Clip regions to intersect, outermost first; each is in symbol units.
    pub clips: Vec<ClipRegion>,
}

/// A clip region: a path already mapped to symbol units.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ClipRegion {
    /// The clip path's segments in symbol units.
    pub segments: Vec<Segment>,
}

/// What a [`DrawItem`] draws.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Shape {
    /// A path: absolute move, line, curve, arc and close segments.
    Path(Vec<Segment>),
    /// A circle.
    Circle {
        /// Centre.
        center: Point,
        /// Radius.
        radius: f64,
    },
    /// A run of text.
    Text(Text),
}

/// A run of text.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Text {
    /// Anchor position (the baseline start, middle or end by [`Text::anchor`]).
    pub position: Point,
    /// The characters.
    pub content: String,
    /// Font size in symbol units.
    pub font_size: f64,
    /// CSS font family list.
    pub font_family: String,
    /// Font weight.
    pub font_weight: FontWeight,
    /// Horizontal alignment relative to [`Text::position`].
    pub anchor: TextAnchor,
    /// Vertical alignment, when the symbol sets one.
    pub baseline: Option<Baseline>,
}

/// Horizontal text alignment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum TextAnchor {
    /// The position is the start of the text.
    Start,
    /// The position is the middle of the text.
    Middle,
    /// The position is the end of the text.
    End,
}

/// Font weight.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FontWeight {
    /// `normal`.
    Normal,
    /// `bold`.
    Bold,
    /// `bolder`.
    Bolder,
    /// `lighter`.
    Lighter,
    /// A number from 100 to 900 in steps of 100.
    Number(u16),
}

/// Vertical text alignment (SVG `dominant-baseline`).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Baseline {
    /// `auto`.
    Auto,
    /// `text-bottom`.
    TextBottom,
    /// `alphabetic`.
    Alphabetic,
    /// `ideographic`.
    Ideographic,
    /// `middle`.
    Middle,
    /// `central`.
    Central,
    /// `mathematical`.
    Mathematical,
    /// `hanging`.
    Hanging,
    /// `text-top`.
    TextTop,
    /// `text-before-edge`.
    TextBeforeEdge,
    /// `text-after-edge`.
    TextAfterEdge,
}

/// A fill or stroke paint.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Paint {
    /// Not painted.
    None,
    /// A CSS colour, as milsymbol.js gives it (`red`, `#f00`,
    /// `rgba(255,255,255,0.4)`, …).
    Solid(Color),
}

/// Stroke line end shape.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineCap {
    /// Flat ends at the endpoint.
    Butt,
    /// Rounded ends.
    Round,
    /// Square ends.
    Square,
}

/// Stroke corner shape.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LineJoin {
    /// Sharp corners.
    Miter,
    /// Rounded corners.
    Round,
}

/// How an item is painted; the values SVG would use for the item.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Appearance {
    /// Fill paint.
    pub fill: Paint,
    /// Fill opacity, 0 to 1.
    pub fill_opacity: f64,
    /// Stroke paint.
    pub stroke: Paint,
    /// Stroke width in symbol units.
    pub stroke_width: f64,
    /// Dash lengths, empty for a solid line.
    pub dashes: Vec<f64>,
    /// Line end shape.
    pub line_cap: LineCap,
    /// Corner shape.
    pub line_join: LineJoin,
}

//! Path geometry: typed segments parsed from SVG path data.

use super::Str;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use core::fmt;

/// A point in symbol units.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (down is positive).
    pub y: f64,
}

/// One absolute path segment.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Segment {
    /// Start a new subpath.
    MoveTo(Point),
    /// Straight line.
    LineTo(Point),
    /// Quadratic Bézier.
    QuadTo {
        /// Control point.
        ctrl: Point,
        /// End point.
        to: Point,
    },
    /// Cubic Bézier.
    CubicTo {
        /// First control point.
        ctrl1: Point,
        /// Second control point.
        ctrl2: Point,
        /// End point.
        to: Point,
    },
    /// Elliptical arc, as in SVG.
    ArcTo {
        /// X radius.
        rx: f64,
        /// Y radius.
        ry: f64,
        /// X-axis rotation in degrees.
        rotation: f64,
        /// Large-arc flag.
        large_arc: bool,
        /// Sweep flag.
        sweep: bool,
        /// End point.
        to: Point,
    },
    /// Close the current subpath.
    Close,
}

/// Path geometry.
///
/// Holds the path-data text, so the SVG serializer can reproduce upstream
/// output byte for byte, and optionally its parsed segments.
/// [`PathData::segments`] gives the typed, absolute geometry that non-SVG
/// renderers consume; it borrows the cached segments when present (see
/// [`PathData::parse`], [`PathData::from_segments`] and
/// [`parse_paths`](crate::ir::parse_paths)) and parses otherwise.
///
/// With the `compact-paths` feature a built-in icon's path stays in its packed
/// form and its text is produced on demand: [`PathData::source`] then builds a
/// new string on every call, while [`PathData::write_source`] and
/// [`PathData::segments`] allocate nothing beyond their result. Cloning never
/// allocates for the path.
#[derive(Debug, Clone)]
pub struct PathData {
    source: Source,
    parsed: Option<Vec<Segment>>,
}

/// Where the path-data text lives: always text, unless `compact-paths` adds
/// the packed form.
#[cfg(not(feature = "compact-paths"))]
type Source = Str;
#[cfg(feature = "compact-paths")]
use compact::Source;

/// Two paths are equal when their path data is equal.
#[cfg(not(feature = "compact-paths"))]
impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

/// Error from [`PathData::segments`], carrying the segments parsed before the
/// error (SVG renderers draw that prefix).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct PathParseError {
    /// Byte offset of the offending input.
    pub offset: usize,
    /// Segments successfully parsed before the error.
    pub valid_prefix: Vec<Segment>,
}

impl fmt::Display for PathParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid path data at byte {}", self.offset)
    }
}

impl core::error::Error for PathParseError {}

impl PathData {
    /// Wraps SVG path-data text (parsed on demand).
    pub fn new(d: impl Into<Str>) -> Self {
        PathData {
            source: Source::from(d.into()),
            parsed: None,
        }
    }

    /// Wraps and parses SVG path-data text, caching the segments.
    pub fn parse(d: impl Into<Str>) -> Result<Self, PathParseError> {
        PathData::new(d).into_parsed()
    }

    /// Builds a path from absolute segments; the path-data text is derived
    /// from them (`M`, `L`, `Q`, `C`, `A`, `Z` with absolute coordinates).
    pub fn from_segments(segments: Vec<Segment>) -> Self {
        let mut d = alloc::string::String::new();
        for (i, seg) in segments.iter().enumerate() {
            if i > 0 {
                d.push(' ');
            }
            write_segment(&mut d, seg);
        }
        PathData {
            source: Source::from(Str::Owned(d)),
            parsed: Some(segments),
        }
    }

    /// Parses and caches the segments, so later [`PathData::segments`] calls
    /// borrow them.
    pub fn into_parsed(mut self) -> Result<Self, PathParseError> {
        self.cache_segments()?;
        Ok(self)
    }

    /// Parses and caches the segments in place; on error the path is left
    /// unchanged.
    pub fn cache_segments(&mut self) -> Result<(), PathParseError> {
        if self.parsed.is_none() {
            self.parsed = Some(self.parse_source()?);
        }
        Ok(())
    }

    /// The path-data text as upstream would serialize it.
    ///
    /// Always [`Cow::Borrowed`] in the default build. With the
    /// `compact-paths` feature a path stored packed decodes into a new
    /// string on every call; writers should use [`PathData::write_source`],
    /// which never allocates. Call [`Cow::into_owned`] for a `String`:
    /// `to_owned()` on a `Cow` is another `Cow`.
    ///
    /// ```
    /// use milsymbol::ir::PathData;
    ///
    /// let d = PathData::new("M0,0 L10,10");
    /// let text: String = d.source().into_owned();
    /// assert_eq!(text, "M0,0 L10,10");
    /// ```
    #[must_use = "with `compact-paths` this allocates; use `write_source` to stream the text"]
    pub fn source(&self) -> Cow<'_, str> {
        self.source_cow()
    }

    /// Writes the path-data text of [`PathData::source`] to `out`. It never
    /// allocates, whatever the feature, so it is the form to use in writers.
    ///
    /// ```
    /// use milsymbol::ir::PathData;
    ///
    /// let mut out = String::new();
    /// PathData::new("M0,0 L10,10").write_source(&mut out)?;
    /// assert_eq!(out, "M0,0 L10,10");
    /// # Ok::<(), std::fmt::Error>(())
    /// ```
    pub fn write_source<W: fmt::Write>(&self, out: &mut W) -> fmt::Result {
        self.write_text(out)
    }

    /// The path as absolute segments (`H`/`V` become lines, smooth curves get
    /// explicit control points). Borrowed when cached, parsed otherwise.
    ///
    /// ```
    /// use milsymbol::ir::{PathData, Point, Segment};
    ///
    /// let d = PathData::new("M10,10 h20 v20 z");
    /// let segments = d.segments()?;
    /// assert_eq!(segments[1], Segment::LineTo(Point { x: 30.0, y: 10.0 }));
    /// assert_eq!(segments.last(), Some(&Segment::Close));
    /// # Ok::<(), milsymbol::ir::PathParseError>(())
    /// ```
    pub fn segments(&self) -> Result<Cow<'_, [Segment]>, PathParseError> {
        match &self.parsed {
            Some(p) => Ok(Cow::Borrowed(p)),
            None => self.parse_source().map(Cow::Owned),
        }
    }
}

fn write_point(d: &mut alloc::string::String, p: Point) {
    crate::js::write_number(d, p.x);
    d.push(',');
    crate::js::write_number(d, p.y);
}

fn write_segment(d: &mut alloc::string::String, seg: &Segment) {
    match *seg {
        Segment::MoveTo(p) => {
            d.push('M');
            write_point(d, p);
        }
        Segment::LineTo(p) => {
            d.push('L');
            write_point(d, p);
        }
        Segment::QuadTo { ctrl, to } => {
            d.push('Q');
            write_point(d, ctrl);
            d.push(' ');
            write_point(d, to);
        }
        Segment::CubicTo { ctrl1, ctrl2, to } => {
            d.push('C');
            write_point(d, ctrl1);
            d.push(' ');
            write_point(d, ctrl2);
            d.push(' ');
            write_point(d, to);
        }
        Segment::ArcTo {
            rx,
            ry,
            rotation,
            large_arc,
            sweep,
            to,
        } => {
            d.push('A');
            write_point(d, Point { x: rx, y: ry });
            d.push(' ');
            crate::js::write_number(d, rotation);
            d.push_str(if large_arc { " 1" } else { " 0" });
            d.push_str(if sweep { " 1 " } else { " 0 " });
            write_point(d, to);
        }
        Segment::Close => d.push('Z'),
    }
}

/// The text accessors of the default build, where every path is text.
#[cfg(not(feature = "compact-paths"))]
impl PathData {
    #[inline(always)]
    fn source_cow(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.source)
    }

    #[inline(always)]
    fn write_text<W: fmt::Write>(&self, out: &mut W) -> fmt::Result {
        out.write_str(&self.source)
    }

    #[inline(always)]
    fn parse_source(&self) -> Result<Vec<Segment>, PathParseError> {
        Parser::new(&self.source).run()
    }

    /// The path-data text.
    #[inline(always)]
    pub(crate) fn text(&self) -> &str {
        &self.source
    }

    /// Whether the text contains `null`, which marks an invalid upstream
    /// symbol.
    #[inline(always)]
    pub(crate) fn contains_null(&self) -> bool {
        self.source.contains("null")
    }
}

#[cfg(feature = "compact-paths")]
pub(crate) mod codec;
#[cfg(feature = "compact-paths")]
mod compact;
#[cfg(feature = "compact-paths")]
pub(crate) mod packed;
#[cfg_attr(feature = "compact-paths", path = "path/parse_generic.rs")]
mod parse;
/// The original text parser, to check the shared grammar against.
#[cfg(all(test, feature = "compact-paths"))]
#[path = "path/parse.rs"]
mod parse_reference;
#[cfg(not(feature = "compact-paths"))]
use parse::Parser;

/// Segments of `text` from the text-only parser.
#[cfg(all(test, feature = "compact-paths"))]
pub(crate) fn reference_segments(text: &str) -> Result<Vec<Segment>, PathParseError> {
    parse_reference::Parser::new(text).run()
}

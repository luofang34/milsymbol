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
#[derive(Debug, Clone)]
pub struct PathData {
    source: Source,
    parsed: Option<Vec<Segment>>,
}

/// Where the path-data text lives.
#[derive(Debug, Clone)]
enum Source {
    Text(Str),
    /// The packed form in the generated tables (see `codec`); the text is
    /// produced on demand.
    #[cfg(feature = "compact-paths")]
    Packed(&'static [u8]),
}

/// Two paths are equal when their path data is equal.
impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        match (&self.source, &other.source) {
            (Source::Text(a), Source::Text(b)) => a == b,
            #[cfg(feature = "compact-paths")]
            (Source::Packed(a), Source::Packed(b)) if a == b => true,
            #[cfg(feature = "compact-paths")]
            (Source::Packed(_), Source::Packed(_)) => self.source() == other.source(),
            #[cfg(feature = "compact-paths")]
            (Source::Packed(_), Source::Text(t)) => self.matches_text(t),
            #[cfg(feature = "compact-paths")]
            (Source::Text(t), Source::Packed(_)) => other.matches_text(t),
        }
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
            source: Source::Text(d.into()),
            parsed: None,
        }
    }

    /// A path that keeps its packed form and produces text and segments on
    /// demand, so building one allocates nothing.
    #[cfg(feature = "compact-paths")]
    pub(crate) fn from_packed(bytes: &'static [u8]) -> Self {
        PathData {
            source: Source::Packed(bytes),
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
            source: Source::Text(Str::Owned(d)),
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
    /// Borrowed unless the path is stored packed (feature `compact-paths`),
    /// where the text is decoded into a new string; use
    /// [`PathData::write_source`] to stream it without allocating.
    pub fn source(&self) -> Cow<'_, str> {
        match &self.source {
            Source::Text(s) => Cow::Borrowed(s),
            #[cfg(feature = "compact-paths")]
            Source::Packed(bytes) => {
                let mut size = Length(0);
                codec::write_text(bytes, &mut size).ok();
                let mut d = alloc::string::String::with_capacity(size.0);
                codec::write_text(bytes, &mut d).ok();
                Cow::Owned(d)
            }
        }
    }

    /// Writes the path-data text of [`PathData::source`] to `out` without
    /// allocating.
    pub fn write_source<W: fmt::Write>(&self, out: &mut W) -> fmt::Result {
        match &self.source {
            Source::Text(s) => out.write_str(s),
            #[cfg(feature = "compact-paths")]
            Source::Packed(bytes) => codec::write_text(bytes, out),
        }
    }

    /// The text when it is stored as arbitrary text, which serializers must
    /// escape. `None` for a packed path, whose text only has the characters
    /// the encoder emits (`cmzvhlMCVHZaAqQsStT`, digits, `.-+eE`, space and
    /// comma), none of which any markup or JSON escaping changes.
    pub(crate) fn as_text(&self) -> Option<&str> {
        match &self.source {
            Source::Text(s) => Some(s),
            #[cfg(feature = "compact-paths")]
            Source::Packed(bytes) => codec::raw_text(bytes),
        }
    }

    /// Whether the text contains `null`, which marks an invalid upstream
    /// symbol.
    pub(crate) fn contains_null(&self) -> bool {
        self.as_text().is_some_and(|t| t.contains("null"))
    }

    #[cfg(feature = "compact-paths")]
    fn matches_text(&self, text: &str) -> bool {
        let mut rest = Matcher(text);
        self.write_source(&mut rest).is_ok() && rest.0.is_empty()
    }

    fn parse_source(&self) -> Result<Vec<Segment>, PathParseError> {
        match &self.source {
            Source::Text(s) => Parser::new(s).run(),
            #[cfg(feature = "compact-paths")]
            Source::Packed(bytes) => match codec::raw_text(bytes) {
                Some(text) => Parser::new(text).run(),
                None => match packed::segments(bytes) {
                    Some(segments) => Ok(segments),
                    None => Parser::new(&self.source()).run(),
                },
            },
        }
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
            None => match &self.source {
                Source::Text(s) => Parser::new(s).run().map(Cow::Owned),
                #[cfg(feature = "compact-paths")]
                Source::Packed(_) => self.parse_source().map(Cow::Owned),
            },
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

/// A sink that only counts the bytes written.
#[cfg(feature = "compact-paths")]
struct Length(usize);

#[cfg(feature = "compact-paths")]
impl fmt::Write for Length {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.wrapping_add(s.len());
        Ok(())
    }
}

#[cfg(feature = "compact-paths")]
pub(crate) mod codec;
#[cfg(feature = "compact-paths")]
pub(crate) mod packed;
mod parse;
use parse::Parser;

/// A sink that succeeds while the text written is a prefix of what remains.
#[cfg(feature = "compact-paths")]
struct Matcher<'a>(&'a str);

#[cfg(feature = "compact-paths")]
impl fmt::Write for Matcher<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.strip_prefix(s).ok_or(fmt::Error)?;
        Ok(())
    }
}

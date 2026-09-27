//! Path geometry: typed segments parsed from SVG path data.

use super::Str;
use alloc::vec::Vec;
use core::fmt;

/// A point in symbol units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (down is positive).
    pub y: f64,
}

/// One absolute path segment.
#[derive(Debug, Clone, Copy, PartialEq)]
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
/// Holds the upstream path-data text so the SVG serializer can reproduce
/// upstream output byte for byte; [`PathData::segments`] gives the typed,
/// absolute geometry that non-SVG renderers consume.
#[derive(Debug, Clone, PartialEq)]
pub struct PathData(Str);

/// Error from [`PathData::segments`], carrying the segments parsed before the
/// error (SVG renderers draw that prefix).
#[derive(Debug, Clone, PartialEq)]
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

#[cfg(feature = "std")]
impl std::error::Error for PathParseError {}

impl PathData {
    /// Wraps SVG path-data text.
    pub fn new(d: impl Into<Str>) -> Self {
        PathData(d.into())
    }

    /// The path-data text as upstream would serialize it.
    pub fn source(&self) -> &str {
        &self.0
    }

    /// Parses the path into absolute segments (`H`/`V` become lines, smooth
    /// curves get explicit control points).
    pub fn segments(&self) -> Result<Vec<Segment>, PathParseError> {
        Parser {
            s: self.0.as_bytes(),
            i: 0,
        }
        .run()
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

struct State {
    out: Vec<Segment>,
    cur: Point,
    start: Point,
    last_ctrl: Option<(u8, Point)>,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    fn skip_sep(&mut self) {
        self.skip_ws();
        if self.s.get(self.i) == Some(&b',') {
            self.i += 1;
            self.skip_ws();
        }
    }

    fn at_number(&mut self) -> bool {
        self.skip_sep();
        self.s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn number(&mut self) -> Option<f64> {
        self.skip_sep();
        let start = self.i;
        let b = self.s;
        if matches!(b.get(self.i), Some(b'+' | b'-')) {
            self.i += 1;
        }
        let mut digits = false;
        while b.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
            digits = true;
        }
        if b.get(self.i) == Some(&b'.') {
            self.i += 1;
            while b.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
                digits = true;
            }
        }
        if !digits {
            self.i = start;
            return None;
        }
        if matches!(b.get(self.i), Some(b'e' | b'E')) {
            let save = self.i;
            self.i += 1;
            if matches!(b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let exp_start = self.i;
            while b.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == exp_start {
                self.i = save;
            }
        }
        core::str::from_utf8(b.get(start..self.i)?)
            .ok()?
            .parse()
            .ok()
    }

    fn flag(&mut self) -> Option<bool> {
        self.skip_sep();
        let f = match self.s.get(self.i)? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.i += 1;
        Some(f)
    }

    fn point(&mut self, rel: bool, cur: Point) -> Option<Point> {
        let x = self.number()?;
        let y = self.number()?;
        Some(if rel {
            Point {
                x: cur.x + x,
                y: cur.y + y,
            }
        } else {
            Point { x, y }
        })
    }

    fn err(&self, st: State) -> PathParseError {
        PathParseError {
            offset: self.i,
            valid_prefix: st.out,
        }
    }

    fn run(mut self) -> Result<Vec<Segment>, PathParseError> {
        let origin = Point { x: 0.0, y: 0.0 };
        let mut st = State {
            out: Vec::new(),
            cur: origin,
            start: origin,
            last_ctrl: None,
        };
        loop {
            self.skip_ws();
            let Some(&c) = self.s.get(self.i) else {
                return Ok(st.out);
            };
            if !c.is_ascii_alphabetic() || (st.out.is_empty() && !matches!(c, b'M' | b'm')) {
                return Err(self.err(st));
            }
            self.i += 1;
            if self.command(c, &mut st).is_none() {
                return Err(self.err(st));
            }
        }
    }

    fn command(&mut self, c: u8, st: &mut State) -> Option<()> {
        let rel = c.is_ascii_lowercase();
        let upper = c.to_ascii_uppercase();
        if upper == b'Z' {
            st.out.push(Segment::Close);
            st.cur = st.start;
            st.last_ctrl = None;
            return Some(());
        }
        let mut first = true;
        loop {
            if !first && !self.at_number() {
                return Some(());
            }
            self.segment(upper, rel, first, st)?;
            first = false;
        }
    }

    fn segment(&mut self, upper: u8, rel: bool, first: bool, st: &mut State) -> Option<()> {
        let cur = st.cur;
        let (seg, ctrl) = match upper {
            b'M' => {
                let p = self.point(rel, cur)?;
                if first {
                    st.start = p;
                    (Segment::MoveTo(p), None)
                } else {
                    (Segment::LineTo(p), None)
                }
            }
            b'L' => (Segment::LineTo(self.point(rel, cur)?), None),
            b'H' => {
                let x = self.number()?;
                (
                    Segment::LineTo(Point {
                        x: if rel { cur.x + x } else { x },
                        y: cur.y,
                    }),
                    None,
                )
            }
            b'V' => {
                let y = self.number()?;
                (
                    Segment::LineTo(Point {
                        x: cur.x,
                        y: if rel { cur.y + y } else { y },
                    }),
                    None,
                )
            }
            b'A' => (self.arc(rel, cur)?, None),
            _ => self.curve(upper, rel, cur, st.last_ctrl)?,
        };
        st.cur = match seg {
            Segment::MoveTo(p) | Segment::LineTo(p) => p,
            Segment::QuadTo { to, .. }
            | Segment::CubicTo { to, .. }
            | Segment::ArcTo { to, .. } => to,
            Segment::Close => st.start,
        };
        st.last_ctrl = ctrl;
        st.out.push(seg);
        Some(())
    }

    /// Bézier commands; returns the segment and its reflectable control point.
    fn curve(
        &mut self,
        upper: u8,
        rel: bool,
        cur: Point,
        last: Option<(u8, Point)>,
    ) -> Option<(Segment, Option<(u8, Point)>)> {
        Some(match upper {
            b'C' => {
                let ctrl1 = self.point(rel, cur)?;
                let ctrl2 = self.point(rel, cur)?;
                let to = self.point(rel, cur)?;
                (Segment::CubicTo { ctrl1, ctrl2, to }, Some((b'C', ctrl2)))
            }
            b'S' => {
                let ctrl1 = reflect(last, b'C', cur);
                let ctrl2 = self.point(rel, cur)?;
                let to = self.point(rel, cur)?;
                (Segment::CubicTo { ctrl1, ctrl2, to }, Some((b'C', ctrl2)))
            }
            b'Q' => {
                let ctrl = self.point(rel, cur)?;
                let to = self.point(rel, cur)?;
                (Segment::QuadTo { ctrl, to }, Some((b'Q', ctrl)))
            }
            b'T' => {
                let ctrl = reflect(last, b'Q', cur);
                let to = self.point(rel, cur)?;
                (Segment::QuadTo { ctrl, to }, Some((b'Q', ctrl)))
            }
            _ => return None,
        })
    }

    fn arc(&mut self, rel: bool, cur: Point) -> Option<Segment> {
        let rx = self.number()?;
        let ry = self.number()?;
        let rotation = self.number()?;
        let large_arc = self.flag()?;
        let sweep = self.flag()?;
        let to = self.point(rel, cur)?;
        Some(Segment::ArcTo {
            rx,
            ry,
            rotation,
            large_arc,
            sweep,
            to,
        })
    }
}

fn reflect(last: Option<(u8, Point)>, kind: u8, cur: Point) -> Point {
    match last {
        Some((k, p)) if k == kind => Point {
            x: 2.0 * cur.x - p.x,
            y: 2.0 * cur.y - p.y,
        },
        _ => cur,
    }
}

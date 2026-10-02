//! SVG path-data parser (SVG 1.1 path grammar) producing absolute segments.

use super::{PathParseError, Point, Segment};
use alloc::vec::Vec;

pub(super) struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    /// The next argument directly follows a command letter, where the SVG
    /// grammar allows whitespace but no comma.
    first_arg: bool,
}

struct State {
    out: Vec<Segment>,
    cur: Point,
    start: Point,
    last_ctrl: Option<(u8, Point)>,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a str) -> Self {
        Parser {
            s: source.as_bytes(),
            i: 0,
            first_arg: false,
        }
    }

    fn skip_ws(&mut self) {
        while self.s.get(self.i).is_some_and(|c| c.is_ascii_whitespace()) {
            self.i += 1;
        }
    }

    /// Index after the separator before the next argument: whitespace,
    /// plus one comma unless the argument follows a command letter.
    fn after_sep(&self) -> usize {
        let ws = |mut i: usize| {
            while self.s.get(i).is_some_and(|c| c.is_ascii_whitespace()) {
                i += 1;
            }
            i
        };
        let i = ws(self.i);
        if !self.first_arg && self.s.get(i) == Some(&b',') {
            ws(i + 1)
        } else {
            i
        }
    }

    fn skip_sep(&mut self) {
        self.i = self.after_sep();
        self.first_arg = false;
    }

    /// Whether another argument follows (without consuming anything).
    fn at_number(&self) -> bool {
        self.s
            .get(self.after_sep())
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
}

impl Lex for Parser<'_> {
    fn next_command(&mut self, first: bool) -> Next {
        self.skip_ws();
        let Some(&c) = self.s.get(self.i) else {
            return Next::End;
        };
        if !c.is_ascii_alphabetic() || (first && !matches!(c, b'M' | b'm')) {
            return Next::Invalid;
        }
        self.i += 1;
        self.first_arg = true;
        Next::Command(c)
    }

    fn at_number(&mut self) -> bool {
        Parser::at_number(self)
    }

    fn number(&mut self) -> Option<f64> {
        Parser::number(self)
    }

    fn flag(&mut self) -> Option<bool> {
        Parser::flag(self)
    }
}

impl Parser<'_> {
    pub(super) fn run(mut self) -> Result<Vec<Segment>, PathParseError> {
        run_with(&mut self).map_err(|valid_prefix| PathParseError {
            offset: self.i,
            valid_prefix,
        })
    }
}

/// The next element of a path as a lexer sees it.
pub(super) enum Next {
    Command(u8),
    End,
    Invalid,
}

/// Token source of the path grammar: SVG text or packed bytes.
pub(super) trait Lex {
    /// The next command letter; `first` requires it to be a move-to.
    fn next_command(&mut self, first: bool) -> Next;
    /// Whether another argument follows (without consuming anything).
    fn at_number(&mut self) -> bool;
    fn number(&mut self) -> Option<f64>;
    fn flag(&mut self) -> Option<bool>;
}

/// Runs the path grammar over `lex`; the error is the segments parsed before
/// the first invalid input.
#[inline(never)]
pub(super) fn run_with<L: Lex>(lex: &mut L) -> Result<Vec<Segment>, Vec<Segment>> {
    let origin = Point { x: 0.0, y: 0.0 };
    let mut st = State {
        out: Vec::new(),
        cur: origin,
        start: origin,
        last_ctrl: None,
    };
    loop {
        let c = match lex.next_command(st.out.is_empty()) {
            Next::End => return Ok(st.out),
            Next::Invalid => return Err(st.out),
            Next::Command(c) => c,
        };
        if command(lex, c, &mut st).is_none() {
            return Err(st.out);
        }
    }
}

fn point<L: Lex>(lex: &mut L, rel: bool, cur: Point) -> Option<Point> {
    let x = lex.number()?;
    let y = lex.number()?;
    Some(if rel {
        Point {
            x: cur.x + x,
            y: cur.y + y,
        }
    } else {
        Point { x, y }
    })
}

fn command<L: Lex>(lex: &mut L, c: u8, st: &mut State) -> Option<()> {
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
        if !first && !lex.at_number() {
            return Some(());
        }
        segment(lex, upper, rel, first, st)?;
        first = false;
    }
}

fn segment<L: Lex>(lex: &mut L, upper: u8, rel: bool, first: bool, st: &mut State) -> Option<()> {
    let cur = st.cur;
    let (seg, ctrl) = match upper {
        b'M' => {
            let p = point(lex, rel, cur)?;
            if first {
                st.start = p;
                (Segment::MoveTo(p), None)
            } else {
                (Segment::LineTo(p), None)
            }
        }
        b'L' => (Segment::LineTo(point(lex, rel, cur)?), None),
        b'H' => {
            let x = lex.number()?;
            (
                Segment::LineTo(Point {
                    x: if rel { cur.x + x } else { x },
                    y: cur.y,
                }),
                None,
            )
        }
        b'V' => {
            let y = lex.number()?;
            (
                Segment::LineTo(Point {
                    x: cur.x,
                    y: if rel { cur.y + y } else { y },
                }),
                None,
            )
        }
        b'A' => (arc(lex, rel, cur)?, None),
        _ => curve(lex, upper, rel, cur, st.last_ctrl)?,
    };
    st.cur = match seg {
        Segment::MoveTo(p) | Segment::LineTo(p) => p,
        Segment::QuadTo { to, .. } | Segment::CubicTo { to, .. } | Segment::ArcTo { to, .. } => to,
        Segment::Close => st.start,
    };
    st.last_ctrl = ctrl;
    st.out.push(seg);
    Some(())
}

/// Bézier commands; returns the segment and its reflectable control point.
fn curve<L: Lex>(
    lex: &mut L,
    upper: u8,
    rel: bool,
    cur: Point,
    last: Option<(u8, Point)>,
) -> Option<(Segment, Option<(u8, Point)>)> {
    Some(match upper {
        b'C' => {
            let ctrl1 = point(lex, rel, cur)?;
            let ctrl2 = point(lex, rel, cur)?;
            let to = point(lex, rel, cur)?;
            (Segment::CubicTo { ctrl1, ctrl2, to }, Some((b'C', ctrl2)))
        }
        b'S' => {
            let ctrl1 = reflect(last, b'C', cur);
            let ctrl2 = point(lex, rel, cur)?;
            let to = point(lex, rel, cur)?;
            (Segment::CubicTo { ctrl1, ctrl2, to }, Some((b'C', ctrl2)))
        }
        b'Q' => {
            let ctrl = point(lex, rel, cur)?;
            let to = point(lex, rel, cur)?;
            (Segment::QuadTo { ctrl, to }, Some((b'Q', ctrl)))
        }
        b'T' => {
            let ctrl = reflect(last, b'Q', cur);
            let to = point(lex, rel, cur)?;
            (Segment::QuadTo { ctrl, to }, Some((b'Q', ctrl)))
        }
        _ => return None,
    })
}

fn arc<L: Lex>(lex: &mut L, rel: bool, cur: Point) -> Option<Segment> {
    let rx = lex.number()?;
    let ry = lex.number()?;
    let rotation = lex.number()?;
    let large_arc = lex.flag()?;
    let sweep = lex.flag()?;
    let to = point(lex, rel, cur)?;
    Some(Segment::ArcTo {
        rx,
        ry,
        rotation,
        large_arc,
        sweep,
        to,
    })
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

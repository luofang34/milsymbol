//! Compact binary encoding of SVG path data, decoded without allocation.
//!
//! The generated tables hold paths in this form. This module has no
//! dependencies beyond `core` so the code generator can include it and check
//! every encoded path against the decoder the library ships.
//!
//! A path is a header byte followed by operations:
//!
//! * header: bit 0 no separator after a command letter, bit 1 ` ` instead of
//!   `,` before odd-numbered arguments (the second of a pair), bit 2 no
//!   separator before a command letter, bits 4..=6 decimal places of scaled
//!   numbers, bit 7 the rest is raw text; other arguments follow a space;
//! * `0xxxxxxx`, `10xxxxxx xxxxxxxx`, `1110xxxx xxxxxxxx xxxxxxxx`: a number
//!   as a zigzag integer scaled by ten to the header's decimal places;
//! * `0xC0..=0xD3`: a command letter from [`LETTERS`];
//! * `0xD4`: the number text `-0`; `0xD5 len text`: any other number text;
//!   `0xD6`: the end of the path, preceded by its trailing separator;
//! * `0xD8..=0xDC`: the next token's separator is `SEPS[op - 0xD8]` instead
//!   of the one the header implies.

use core::fmt;

/// Command letters, in operation order.
pub(crate) const LETTERS: &[u8; 20] = b"cmzvhlLMCVHZaAqQsStT";

/// Separators an override operation can select.
pub(crate) const SEPS: [&str; 5] = ["", " ", ",", "  ", ", "];

pub(crate) const HDR_FIRST_TIGHT: u8 = 1;
pub(crate) const HDR_ODD_SPACE: u8 = 2;
pub(crate) const HDR_LETTER_TIGHT: u8 = 4;
pub(crate) const HDR_PLACES_SHIFT: u32 = 4;
pub(crate) const HDR_RAW: u8 = 0x80;

pub(crate) const OP_LETTER: u8 = 0xC0;
pub(crate) const OP_NEG_ZERO: u8 = 0xD4;
pub(crate) const OP_RAW_NUMBER: u8 = 0xD5;
pub(crate) const OP_TAIL: u8 = 0xD6;
pub(crate) const OP_SEP: u8 = 0xD8;

const POW10: [u32; 7] = [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000];

/// A number token.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Number<'a> {
    /// `n / 10^places`, printed without trailing zeros.
    Scaled { n: i32, places: u8 },
    /// The text `-0`.
    NegZero,
    /// Number text kept verbatim.
    Raw(&'a str),
}

/// A token of a path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Token<'a> {
    /// A command letter.
    Command(u8),
    /// An argument.
    Number(Number<'a>),
    /// The end of the path, after a trailing separator.
    Tail,
}

/// A token and the separator that precedes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Item<'a> {
    pub sep: &'static str,
    pub token: Token<'a>,
}

/// Iterator over the tokens of one encoded path.
#[derive(Debug, Clone)]
pub(crate) struct Items<'a> {
    bytes: &'a [u8],
    header: u8,
    pos: usize,
    arg: u32,
    started: bool,
}

/// The text of a path stored in raw mode.
pub(crate) fn raw_text(bytes: &[u8]) -> Option<&str> {
    let (&header, rest) = bytes.split_first()?;
    if header & HDR_RAW == 0 {
        return None;
    }
    core::str::from_utf8(rest).ok()
}

impl<'a> Items<'a> {
    /// Tokens of the encoded path `bytes` (none for a raw-mode path).
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        let header = bytes.first().copied().unwrap_or(HDR_RAW);
        Items {
            bytes,
            header,
            pos: 1,
            arg: 0,
            started: false,
        }
    }

    fn places(&self) -> u8 {
        (self.header >> HDR_PLACES_SHIFT) & 7
    }

    fn byte(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        self.pos += 1;
        b
    }

    fn default_sep(&self, is_letter: bool) -> &'static str {
        let h = self.header;
        if is_letter {
            return match (self.started, h & HDR_LETTER_TIGHT != 0) {
                (false, _) | (_, true) => "",
                _ => " ",
            };
        }
        match self.arg {
            0 if h & HDR_FIRST_TIGHT != 0 => "",
            0 => " ",
            n if n & 1 == 0 => " ",
            _ if h & HDR_ODD_SPACE != 0 => " ",
            _ => ",",
        }
    }

    fn scaled(&mut self, first: u8) -> Option<Number<'a>> {
        let z = match first {
            0x00..=0x7F => u32::from(first),
            0x80..=0xBF => u32::from(first & 0x3F) << 8 | u32::from(self.byte()?),
            _ => {
                let hi = u32::from(first & 0x0F) << 16;
                hi | u32::from(self.byte()?) << 8 | u32::from(self.byte()?)
            }
        };
        let half = i32::try_from(z >> 1).ok()?;
        let n = if z & 1 == 0 { half } else { -half - 1 };
        Some(Number::Scaled {
            n,
            places: self.places(),
        })
    }

    fn raw_number(&mut self) -> Option<Number<'a>> {
        let len = usize::from(self.byte()?);
        let text = self.bytes.get(self.pos..self.pos.checked_add(len)?)?;
        self.pos += len;
        Some(Number::Raw(core::str::from_utf8(text).ok()?))
    }
}

impl<'a> Iterator for Items<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Item<'a>> {
        if self.header & HDR_RAW != 0 {
            return None;
        }
        let mut forced = None;
        loop {
            let op = self.byte()?;
            let token = match op {
                OP_SEP..=0xDC => {
                    forced = SEPS.get(usize::from(op - OP_SEP)).copied();
                    continue;
                }
                OP_NEG_ZERO => Token::Number(Number::NegZero),
                OP_RAW_NUMBER => Token::Number(self.raw_number()?),
                OP_TAIL => Token::Tail,
                0xC0..=0xD3 => {
                    let letter = LETTERS.get(usize::from(op - OP_LETTER)).copied()?;
                    Token::Command(letter)
                }
                0xD7 | 0xDD..=0xDF | 0xF0..=0xFF => return None,
                _ => Token::Number(self.scaled(op)?),
            };
            let is_letter = matches!(token, Token::Command(_));
            let sep = forced.unwrap_or_else(|| match token {
                Token::Tail => "",
                _ => self.default_sep(is_letter),
            });
            if is_letter {
                self.arg = 0;
            } else if token != Token::Tail {
                self.arg = self.arg.wrapping_add(1);
            }
            self.started = true;
            return Some(Item { sep, token });
        }
    }
}

/// Output staging: the sink receives text in blocks rather than per token.
const BUF: usize = 256;

/// Longest text one non-raw token adds.
const TOKEN_MAX: usize = 32;

/// Staged output text.
struct Buf {
    bytes: [u8; BUF],
    len: usize,
}

impl Buf {
    fn push(&mut self, c: u8) {
        if let Some(slot) = self.bytes.get_mut(self.len) {
            *slot = c;
            self.len += 1;
        }
    }

    fn push_str(&mut self, s: &str) {
        s.bytes().for_each(|c| self.push(c));
    }

    /// Decimal digits of `v`, zero-padded to at least `width`.
    fn push_digits(&mut self, v: u32, width: u32) {
        if width <= 1 && v < 100 {
            if v >= 10 {
                self.push(b'0' + (v / 10) as u8);
            }
            return self.push(b'0' + (v % 10) as u8);
        }
        let mut digits = [b'0'; 10];
        let mut v = v;
        let mut used = 0usize;
        for slot in digits.iter_mut().rev() {
            *slot = b'0' + (v % 10) as u8;
            v /= 10;
            used += 1;
            if v == 0 && used >= width as usize {
                break;
            }
        }
        if let Some(text) = digits.get(digits.len() - used..) {
            text.iter().for_each(|&c| self.push(c));
        }
    }

    fn push_scaled(&mut self, n: i32, places: u8) {
        // Constant divisors for the places almost every number uses.
        if places <= 1 {
            let abs = n.unsigned_abs();
            if n < 0 {
                self.push(b'-');
            }
            if places == 0 {
                return self.push_digits(abs, 0);
            }
            self.push_digits(abs / 10, 0);
            let frac = abs % 10;
            if frac != 0 {
                self.push(b'.');
                self.push(b'0' + frac as u8);
            }
            return;
        }
        let Some(&scale) = POW10.get(usize::from(places)) else {
            return;
        };
        let abs = n.unsigned_abs();
        if n < 0 {
            self.push(b'-');
        }
        self.push_digits(abs / scale, 0);
        let mut frac = abs % scale;
        if frac == 0 {
            return;
        }
        let mut width = u32::from(places);
        while frac.checked_rem(10) == Some(0) {
            frac /= 10;
            width -= 1;
        }
        self.push(b'.');
        self.push_digits(frac, width);
    }

    fn as_str(&self) -> &str {
        self.bytes
            .get(..self.len)
            .and_then(|b| core::str::from_utf8(b).ok())
            .unwrap_or("")
    }
}

/// Writes the path text of the encoded path `bytes`.
pub(crate) fn write_text<W: fmt::Write>(bytes: &[u8], out: &mut W) -> fmt::Result {
    if let Some(text) = raw_text(bytes) {
        return out.write_str(text);
    }
    let mut buf = Buf {
        bytes: [0; BUF],
        len: 0,
    };
    for item in Items::new(bytes) {
        if buf.len > BUF - TOKEN_MAX {
            out.write_str(buf.as_str())?;
            buf.len = 0;
        }
        buf.push_str(item.sep);
        match item.token {
            Token::Command(c) => buf.push(c),
            Token::Number(Number::NegZero) => buf.push_str("-0"),
            Token::Number(Number::Scaled { n, places }) => buf.push_scaled(n, places),
            Token::Number(Number::Raw(text)) => {
                out.write_str(buf.as_str())?;
                buf.len = 0;
                out.write_str(text)?;
            }
            Token::Tail => {}
        }
    }
    out.write_str(buf.as_str())
}

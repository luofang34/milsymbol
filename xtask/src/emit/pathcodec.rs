//! Encoder for the packed path-data format decoded by `src/ir/path/codec.rs`.
//!
//! Every encoded path is decoded again with the library's own decoder and must
//! reproduce the source text byte for byte; a path the format cannot express
//! is stored as raw text.

use crate::Error;

#[path = "../../../src/ir/path/codec.rs"]
#[allow(dead_code)]
pub(super) mod codec;

use codec::{
    HDR_FIRST_TIGHT, HDR_LETTER_TIGHT, HDR_ODD_SPACE, HDR_PLACES_SHIFT, HDR_RAW, LETTERS,
    OP_LETTER, OP_NEG_ZERO, OP_RAW_NUMBER, OP_SEP, OP_TAIL, SEPS,
};

/// Decimal places a scaled number can carry.
const MAX_PLACES: u8 = 6;

/// Packed paths and the offset of each in the byte blob.
pub(super) struct PathBlob {
    pub bytes: Vec<u8>,
    pub offsets: Vec<u32>,
    /// Paths stored as raw text because the format cannot express them.
    pub raw_paths: usize,
}

enum Tok<'a> {
    Command(u8),
    Number(&'a str),
    Tail,
}

struct Lexed<'a> {
    sep: usize,
    tok: Tok<'a>,
}

fn sep_index(run: &str) -> Option<usize> {
    SEPS.iter().position(|s| *s == run)
}

fn number_end(b: &[u8], start: usize) -> Option<usize> {
    let digits = |from: usize| {
        b.get(from..)
            .map_or(0, |r| r.iter().take_while(|c| c.is_ascii_digit()).count())
    };
    let mut i = start + usize::from(b.get(start) == Some(&b'-'));
    let int = digits(i);
    i += int;
    if b.get(i) == Some(&b'.') {
        let frac = digits(i + 1);
        if frac == 0 {
            return None;
        }
        i += 1 + frac;
    } else if int == 0 {
        return None;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(i + 1), Some(b'+' | b'-')));
        let exp = digits(i + 1 + sign);
        if exp > 0 {
            i += 1 + sign + exp;
        }
    }
    Some(i)
}

fn lex(text: &str) -> Option<Vec<Lexed<'_>>> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let rest = b.get(i..)?;
        let sep_len = rest.iter().take_while(|c| matches!(c, b' ' | b',')).count();
        let sep = sep_index(text.get(i..i + sep_len)?)?;
        i += sep_len;
        let Some(&c) = b.get(i) else {
            out.push(Lexed {
                sep,
                tok: Tok::Tail,
            });
            break;
        };
        if c.is_ascii_alphabetic() {
            LETTERS.contains(&c).then_some(())?;
            out.push(Lexed {
                sep,
                tok: Tok::Command(c),
            });
            i += 1;
        } else {
            let end = number_end(b, i)?;
            out.push(Lexed {
                sep,
                tok: Tok::Number(text.get(i..end)?),
            });
            i = end;
        }
    }
    Some(out)
}

/// `Some(n)` when `t` is a canonical decimal with at most `places` places,
/// scaled by ten to the `places`.
fn scaled(t: &str, places: u8) -> Option<i64> {
    let (neg, body) = t.strip_prefix('-').map_or((false, t), |r| (true, r));
    let (int, frac) = body.split_once('.').unwrap_or((body, ""));
    let canonical_int = int == "0" || (!int.starts_with('0') && !int.is_empty());
    let canonical_frac = frac.is_empty() || !frac.ends_with('0');
    let digits = |s: &str| s.bytes().all(|c| c.is_ascii_digit());
    if !(canonical_int && canonical_frac && digits(int) && digits(frac)) {
        return None;
    }
    if frac.len() > usize::from(places) {
        return None;
    }
    let padded = format!(
        "{int}{frac}{}",
        "0".repeat(usize::from(places) - frac.len())
    );
    let v: i64 = padded.parse().ok()?;
    Some(if neg { -v } else { v })
}

fn push_number(out: &mut Vec<u8>, n: i64) -> bool {
    let doubled = if n >= 0 {
        n.checked_mul(2)
    } else {
        n.checked_mul(-2).and_then(|v| v.checked_sub(1))
    };
    let Some(z) = doubled else {
        return false;
    };
    match z {
        0..=0x7F => out.push(z as u8),
        0x80..=0x3FFF => out.extend([0x80 | (z >> 8) as u8, z as u8]),
        0x4000..=0xF_FFFF => out.extend([0xE0 | (z >> 16) as u8, (z >> 8) as u8, z as u8]),
        _ => return false,
    }
    true
}

fn default_sep(header: u8, arg: u32, letter: bool, started: bool) -> usize {
    if letter {
        return usize::from(started && header & HDR_LETTER_TIGHT == 0);
    }
    match arg {
        0 => usize::from(header & HDR_FIRST_TIGHT == 0),
        n if n % 2 == 0 => 1,
        _ if header & HDR_ODD_SPACE != 0 => 1,
        _ => 2,
    }
}

fn encode_with(toks: &[Lexed<'_>], header: u8) -> Option<Vec<u8>> {
    let places = (header >> HDR_PLACES_SHIFT) & 7;
    let mut out = vec![header];
    let (mut arg, mut started) = (0u32, false);
    for t in toks {
        let letter = matches!(t.tok, Tok::Command(_));
        let default = match t.tok {
            Tok::Tail => 0,
            _ => default_sep(header, arg, letter, started),
        };
        if t.sep != default {
            out.push(OP_SEP + u8::try_from(t.sep).ok()?);
        }
        match t.tok {
            Tok::Command(c) => {
                let idx = LETTERS.iter().position(|l| *l == c)?;
                out.push(OP_LETTER + u8::try_from(idx).ok()?);
                arg = 0;
            }
            Tok::Tail => out.push(OP_TAIL),
            Tok::Number(text) => {
                let packed = scaled(text, places).filter(|_| text != "-0");
                if text == "-0" {
                    out.push(OP_NEG_ZERO);
                } else if !packed.is_some_and(|n| push_number(&mut out, n)) {
                    out.push(OP_RAW_NUMBER);
                    out.push(u8::try_from(text.len()).ok()?);
                    out.extend(text.bytes());
                }
                arg = arg.wrapping_add(1);
            }
        }
        started = true;
    }
    Some(out)
}

fn raw(text: &str) -> Vec<u8> {
    let mut out = vec![HDR_RAW];
    out.extend(text.bytes());
    out
}

fn encode(text: &str) -> (Vec<u8>, bool) {
    let Some(toks) = lex(text) else {
        return (raw(text), true);
    };
    let mut best: Option<Vec<u8>> = None;
    for places in 0..=MAX_PLACES {
        for style in 0..8u8 {
            let header = style | places << HDR_PLACES_SHIFT;
            if let Some(enc) = encode_with(&toks, header)
                && best.as_ref().is_none_or(|b| enc.len() < b.len())
            {
                best = Some(enc);
            }
        }
    }
    match best {
        Some(b) => (b, false),
        None => (raw(text), true),
    }
}

fn decoded(bytes: &[u8]) -> Result<String, Error> {
    let mut s = String::new();
    codec::write_text(bytes, &mut s).map_err(|_| "decoder failed")?;
    Ok(s)
}

/// Encodes `paths`, checking each against the library decoder.
pub(super) fn pack_paths(paths: &[String]) -> Result<PathBlob, Error> {
    pack_with(paths, encode)
}

/// [`pack_paths`] with the encoder supplied, so the round-trip check can be
/// tested against an encoder that is wrong.
fn pack_with(
    paths: &[String],
    encoder: impl Fn(&str) -> (Vec<u8>, bool),
) -> Result<PathBlob, Error> {
    let mut blob = PathBlob {
        bytes: Vec::new(),
        offsets: Vec::with_capacity(paths.len() + 1),
        raw_paths: 0,
    };
    for text in paths {
        let (enc, is_raw) = encoder(text);
        if decoded(&enc)? != *text {
            return Err(format!("path does not round-trip: {text}").into());
        }
        blob.raw_paths += usize::from(is_raw);
        blob.offsets
            .push(u32::try_from(blob.bytes.len()).map_err(|_| "path blob exceeds u32")?);
        blob.bytes.extend(enc);
    }
    blob.offsets
        .push(u32::try_from(blob.bytes.len()).map_err(|_| "path blob exceeds u32")?);
    Ok(blob)
}

/// A Rust byte-string literal of `bytes`.
pub(super) fn byte_literal(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2 + 3);
    out.push_str("b\"");
    for &b in bytes {
        match b {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            0x20..=0x7E => out.push(char::from(b)),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests;

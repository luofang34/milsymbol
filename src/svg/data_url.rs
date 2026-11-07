//! `data:image/svg+xml;utf8,…` serialization (milsymbol.js `toDataURL()`),
//! encoded in place inside the caller's `String` buffer without duplicating
//! the SVG writer or allocating a second buffer.

use super::{Node, SvgFrame, SvgOptions};
use alloc::string::String;

/// Prefix of milsymbol.js `Symbol.prototype.toDataURL()`.
const DATA_URL_PREFIX: &str = "data:image/svg+xml;utf8,";

/// Appends `data:image/svg+xml;utf8,` followed by the `encodeURIComponent`-encoded
/// SVG document to `out`.
pub(crate) fn render_data_url_into(
    frame: &SvgFrame,
    nodes: &[Node],
    options: &SvgOptions,
    out: &mut String,
) {
    out.push_str(DATA_URL_PREFIX);
    let svg_start = out.len();
    super::render_into(frame, nodes, options, out);
    encode_tail_in_place(out, svg_start);
}

/// Percent-encodes `out[start..]` in place according to ECMAScript
/// `encodeURIComponent`, reusing `out`'s allocation when its capacity fits.
fn encode_tail_in_place(out: &mut String, start: usize) {
    let escaped = out.as_bytes().get(start..).map_or(0, |tail| {
        tail.iter().filter(|&&b| !is_uri_unescaped(b)).count()
    });
    if escaped == 0 {
        return;
    }
    let mut bytes = core::mem::take(out).into_bytes();
    let raw_end = bytes.len();
    let new_len = raw_end + 2 * escaped;
    bytes.resize(new_len, 0);
    let mut w = new_len;
    let mut r = raw_end;
    while r > start {
        r -= 1;
        let b = bytes.get(r).copied().unwrap_or(0);
        if is_uri_unescaped(b) {
            w -= 1;
            if let Some(dst) = bytes.get_mut(w) {
                *dst = b;
            }
        } else {
            w -= 3;
            if let Some(dst) = bytes.get_mut(w..w + 3) {
                dst.copy_from_slice(&[b'%', hex_upper(b >> 4), hex_upper(b)]);
            }
        }
    }
    // `bytes[..start]` came from `out: String` (at char boundary `start`) and
    // `bytes[start..]` now holds only ASCII bytes (< 0x80).
    if let Ok(encoded) = String::from_utf8(bytes) {
        *out = encoded;
    }
}

/// ECMAScript `uriUnescaped`: `A-Z a-z 0-9 - _ . ! ~ * ' ( )`.
#[inline]
const fn is_uri_unescaped(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
        )
}

#[inline]
const fn hex_upper(nibble: u8) -> u8 {
    match nibble & 0x0F {
        n @ 0..=9 => b'0' + n,
        n => b'A' + (n - 10),
    }
}

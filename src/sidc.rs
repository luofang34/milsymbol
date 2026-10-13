//! SIDC interpretation: numeric (2525D/E, APP-6D/E) and legacy letter
//! (2525B/C, APP-6B) symbol identification codes.
//!
//! Rendering never fails on a malformed code: like upstream, it produces
//! metadata that marks the symbol invalid. [`Sidc::parse`] validates a code
//! strictly and exposes its fields as typed values.

use crate::ir::Str;
use crate::js;
use crate::metadata::Metadata;
use crate::small_str::SmallStr;
use alloc::string::String;

mod letter;
mod number;
mod parse;

pub use parse::{
    LetterSidc, ModifierCode, NumericSidc, Sidc, SidcCheckError, SidcError, SidcModifyError,
};

/// Dash arrays used for not-present frames.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Dashes<'a> {
    pub pending: &'a Str,
    pub anticipated: &'a Str,
}

/// Inputs to SIDC interpretation besides the code itself.
pub(crate) struct ParseInput<'a> {
    /// Read version `16` SIDCs as APP-6(E) Version 2.
    pub app6e2: bool,
    pub style_frame: bool,
    pub alternate_medal: bool,
    pub dashes: Dashes<'a>,
}

/// Normalizes a SIDC as upstream does (`*` → `-`, spaces removed), storing
/// standard-length codes inline without allocating.
pub(crate) fn normalize(sidc: &str) -> SmallStr<30> {
    if !sidc.bytes().any(|b| b == b' ' || b == b'*') {
        return SmallStr::from_str_slice(sidc);
    }
    if sidc.is_ascii() && sidc.len() <= 30 {
        let mut buf = [0u8; 30];
        let mut n = 0usize;
        for &b in sidc.as_bytes() {
            if b != b' ' {
                if let Some(slot) = buf.get_mut(n) {
                    *slot = if b == b'*' { b'-' } else { b };
                }
                n += 1;
            }
        }
        if let Ok(len) = u8::try_from(n) {
            return SmallStr::from_utf8_buf(buf, len);
        }
    }
    let mut out = String::with_capacity(sidc.len());
    for c in sidc.chars() {
        match c {
            ' ' => {}
            '*' => out.push('-'),
            _ => out.push(c),
        }
    }
    SmallStr::from_string(out)
}

/// Whether upstream treats the (normalized) SIDC as numeric.
pub(crate) fn is_number_sidc(sidc: &str) -> bool {
    !js::is_nan_str(&js::substr(sidc, 0, 2))
}

/// Interprets `sidc` (already normalized) into `md`. Returns the SIDC as
/// upstream stores it afterwards (letter codes are upper-cased).
pub(crate) fn interpret(
    mut sidc: SmallStr<30>,
    md: &mut Metadata,
    input: &ParseInput<'_>,
) -> SmallStr<30> {
    md.number_sidc = is_number_sidc(&sidc);
    if md.number_sidc {
        number::interpret(&sidc, md, input);
        sidc
    } else {
        if sidc.as_str().is_ascii() {
            sidc.make_ascii_uppercase();
        } else {
            sidc = SmallStr::from_string(sidc.as_str().to_uppercase());
        }
        letter::interpret(&sidc, md, input);
        sidc
    }
}

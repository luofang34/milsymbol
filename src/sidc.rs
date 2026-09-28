//! SIDC interpretation: numeric (2525D/E, APP-6D/E) and legacy letter
//! (2525B/C, APP-6B) symbol identification codes.
//!
//! Rendering never fails on a malformed code: like upstream, it produces
//! metadata that marks the symbol invalid. [`Sidc::parse`] validates a code
//! strictly and exposes its fields as typed values.

use crate::js;
use crate::metadata::Metadata;
use alloc::string::String;

mod letter;
mod number;
mod parse;

pub use parse::{LetterSidc, NumericSidc, Sidc, SidcCheckError, SidcError};

/// Dash arrays used for not-present frames.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Dashes<'a> {
    pub pending: &'a str,
    pub anticipated: &'a str,
}

/// Inputs to SIDC interpretation besides the code itself.
pub(crate) struct ParseInput<'a> {
    pub style_frame: bool,
    pub alternate_medal: bool,
    pub dashes: Dashes<'a>,
}

/// Normalizes a SIDC as upstream does (`*` → `-`, spaces removed).
pub(crate) fn normalize(sidc: &str) -> String {
    sidc.chars()
        .filter(|&c| c != ' ')
        .map(|c| if c == '*' { '-' } else { c })
        .collect()
}

/// Whether upstream treats the (normalized) SIDC as numeric.
pub(crate) fn is_number_sidc(sidc: &str) -> bool {
    !js::is_nan_str(&js::substr(sidc, 0, 2))
}

/// Interprets `sidc` (already normalized) into `md`. Returns the SIDC as
/// upstream stores it afterwards (letter codes are upper-cased).
pub(crate) fn interpret(sidc: &str, md: &mut Metadata, input: &ParseInput<'_>) -> String {
    md.number_sidc = is_number_sidc(sidc);
    if md.number_sidc {
        number::interpret(sidc, md, input);
        String::from(sidc)
    } else {
        let upper = sidc.to_uppercase();
        letter::interpret(&upper, md, input);
        upper
    }
}

//! SIDC interpretation: numeric (2525D/E, APP-6D/E) and legacy letter
//! (2525B/C, APP-6B) symbol identification codes.
//!
//! Rendering never fails on a malformed code: like upstream, it produces
//! metadata that marks the symbol invalid. [`Sidc::parse`] validates a code
//! strictly and exposes its fields as typed values;
//! [`Renderer::check_sidc`](crate::Renderer::check_sidc) also checks that a
//! renderer, extensions included, can draw every part the code names.
//!
//! ```
//! use milsymbol::Renderer;
//! use milsymbol::domain::{Echelon, StandardIdentity};
//! use milsymbol::sidc::{Sidc, SidcCheckError};
//!
//! let Sidc::Numeric(sidc) = Sidc::parse("10031000161211000000")? else {
//!     return Ok(()); // a 20-digit code is always numeric
//! };
//! assert_eq!(sidc.standard_identity(), StandardIdentity::Friend);
//! assert_eq!(sidc.amplifier(), "16"); // battalion
//! assert!(Sidc::parse("10091000001211000000").is_err()); // no identity 9
//!
//! let renderer = Renderer::default();
//! assert!(renderer.check_sidc("10031000161211000000").is_ok());
//! assert!(matches!(
//!     renderer.check_sidc("10031000009999990000"), // no such entity
//!     Err(SidcCheckError::Unsupported { .. })
//! ));
//!
//! // A rendered symbol describes itself with typed values.
//! let symbol = renderer.symbol("10031000161211000000").render()?;
//! assert_eq!(symbol.metadata().echelon, Some(Echelon::BattalionSquadron));
//! assert!(symbol.sidc_validity().is_valid());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Editing
//!
//! A parsed [`Sidc`] changes status, affiliation, identity and context
//! without string surgery, handling the differences between numeric and
//! letter codes. The result is always a valid SIDC whose identity, status and
//! context getters return what was set:
//!
//! ```
//! use milsymbol::Renderer;
//! use milsymbol::domain::{Affiliation, Context, Status};
//! use milsymbol::sidc::Sidc;
//!
//! let friendly = Sidc::parse("10031000001211000000")?;
//! let hostile_damaged = friendly
//!     .with_affiliation(Affiliation::Hostile)
//!     .with_status(Status::Damaged);
//! assert_eq!(hostile_damaged.as_str(), "10061030001211000000");
//!
//! // In an exercise, Hostile is written as Faker; letter codes carry the
//! // context in the identity letter.
//! let exercise = friendly.with_context(Context::Exercise)?.with_affiliation(Affiliation::Hostile);
//! assert_eq!(exercise.as_str(), "10161000001211000000");
//! let letter = Sidc::parse("SFGPUCI----D")?.with_context(Context::Exercise)?;
//! assert_eq!(letter.as_str(), "SDGPUCI----D");
//!
//! let symbol = Renderer::default().symbol(&hostile_damaged).render()?;
//! assert_eq!(symbol.metadata().affiliation, Some(Affiliation::Hostile));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! An exercise Joker or Faker stands for a friendly force playing the suspect
//! or hostile role, so it is drawn with a friendly frame and a marker:
//! `metadata().affiliation` is `Friend` and `base_affiliation` is the role
//! played. [`Sidc::with_standard_identity`] sets the exact identity and fails
//! with [`SidcModifyError`] where the scheme has no code for it (Joker outside
//! an exercise, or Hostile inside one). Letter codes have no simulation
//! context.

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
    /// Interpret the published standards instead of the upstream compatibility rules.
    pub standards: bool,
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

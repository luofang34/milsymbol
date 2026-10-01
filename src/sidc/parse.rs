//! Strict, typed SIDC parsing.
//!
//! Rendering accepts any string, as milsymbol.js does. [`Sidc::parse`]
//! instead checks that every field holds a code the standards define and
//! reports the first problem with its position. Whether a renderer can draw
//! the symbol (it has the icon, including icons added by extensions) is a
//! separate question, answered by
//! [`Renderer::check_sidc`](crate::Renderer::check_sidc).

use crate::domain::{Affiliation, Context, StandardIdentity, Status};
use alloc::string::String;
use core::fmt;
use core::str::FromStr;

mod letter;
mod numeric;

pub use letter::LetterSidc;
pub use numeric::{ModifierCode, NumericSidc};

/// A SIDC whose fields are all well formed. Stored inline, so it is `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sidc {
    /// Numeric SIDC (MIL-STD-2525D/E, APP-6D/E).
    Numeric(NumericSidc),
    /// Letter SIDC (MIL-STD-2525B/C, APP-6B).
    Letter(LetterSidc),
}

/// Why a SIDC is malformed. Positions are 1-based character positions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SidcError {
    /// The SIDC is empty.
    Empty,
    /// Wrong length (numeric SIDCs have 20 or 30 digits, letter SIDCs 10–15 characters).
    Length {
        /// Actual length in characters.
        len: usize,
    },
    /// A character or field value is not allowed at this position.
    InvalidField {
        /// Field name, e.g. `"standard identity"`.
        field: &'static str,
        /// First position of the field.
        position: usize,
        /// The offending text.
        value: String,
    },
}

impl fmt::Display for SidcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SidcError::Empty => f.write_str("empty SIDC"),
            SidcError::Length { len } => write!(f, "SIDC has {len} characters"),
            SidcError::InvalidField {
                field,
                position,
                value,
            } => {
                write!(f, "invalid {field} {value:?} at position {position}")
            }
        }
    }
}

impl core::error::Error for SidcError {}

/// Why a SIDC field cannot be changed to the requested value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SidcModifyError {
    /// The coding scheme has no code for this identity in this context, for
    /// example Joker outside exercises, Suspect or Hostile inside exercises
    /// (use Joker and Faker there), or "none specified" in a numeric SIDC.
    Unrepresentable {
        /// The requested identity.
        identity: StandardIdentity,
        /// The context the SIDC would have.
        context: Context,
    },
    /// The coding scheme does not have this context (letter SIDCs have no
    /// simulation context).
    UnsupportedContext {
        /// The requested context.
        context: Context,
    },
}

impl fmt::Display for SidcModifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SidcModifyError::Unrepresentable { identity, context } => {
                write!(f, "no code for {identity:?} in the {context:?} context")
            }
            SidcModifyError::UnsupportedContext { context } => {
                write!(f, "the {context:?} context is not available in this SIDC")
            }
        }
    }
}

impl core::error::Error for SidcModifyError {}

/// Why a renderer cannot draw a SIDC as a fully recognised symbol.
#[derive(Debug)]
#[non_exhaustive]
pub enum SidcCheckError {
    /// The SIDC is malformed.
    Malformed(SidcError),
    /// Rendering the SIDC failed (milsymbol.js throws for it).
    Render(crate::RenderError),
    /// The SIDC is well formed, but the renderer does not recognise all of
    /// it (for example, it has no icon for the entity).
    Unsupported {
        /// Every reason, as [`Symbol::validity`](crate::Symbol::validity) judges it.
        issues: alloc::vec::Vec<crate::ValidityIssue>,
    },
}

impl fmt::Display for SidcCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SidcCheckError::Malformed(e) => write!(f, "malformed SIDC: {e}"),
            SidcCheckError::Render(e) => write!(f, "SIDC cannot be rendered: {e}"),
            SidcCheckError::Unsupported { issues } => {
                write!(f, "SIDC not supported by this renderer: {issues:?}")
            }
        }
    }
}

impl core::error::Error for SidcCheckError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            SidcCheckError::Malformed(e) => Some(e),
            SidcCheckError::Render(e) => Some(e),
            SidcCheckError::Unsupported { .. } => None,
        }
    }
}

impl From<SidcError> for SidcCheckError {
    fn from(e: SidcError) -> Self {
        SidcCheckError::Malformed(e)
    }
}

fn invalid(field: &'static str, position: usize, value: &str) -> SidcError {
    SidcError::InvalidField {
        field,
        position,
        value: String::from(value),
    }
}

/// Up to `N` ASCII characters stored inline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Code<const N: usize> {
    bytes: [u8; N],
    len: u8,
}

impl<const N: usize> Code<N> {
    /// Collects the non-space characters of `s`, mapped by `map`; rejects
    /// non-ASCII characters and inputs longer than `N`.
    fn collect(s: &str, map: impl Fn(char) -> char) -> Result<Self, SidcError> {
        let mut code = Code {
            bytes: [0; N],
            len: 0,
        };
        let mut len = 0usize;
        for c in s.chars().filter(|&c| c != ' ') {
            let c = map(c);
            len = len.saturating_add(1);
            if !c.is_ascii() {
                return Err(invalid("character", len, c.encode_utf8(&mut [0; 4])));
            }
            if let Some(slot) = code.bytes.get_mut(len - 1) {
                *slot = c as u8;
            }
        }
        if len > N {
            return Err(SidcError::Length { len });
        }
        code.len = u8::try_from(len).map_err(|_| SidcError::Length { len })?;
        Ok(code)
    }

    /// A copy with the character at `position` (1-based) replaced. The
    /// callers pass only codes their field allows, so the result stays valid.
    fn with_char(mut self, position: usize, c: char) -> Self {
        let in_code = position >= 1 && position <= usize::from(self.len);
        if let (true, true, Some(slot)) = (
            in_code,
            c.is_ascii(),
            self.bytes.get_mut(position.wrapping_sub(1)),
        ) {
            *slot = c as u8;
        }
        self
    }

    fn as_str(&self) -> &str {
        self.bytes
            .get(..usize::from(self.len))
            .and_then(|b| core::str::from_utf8(b).ok())
            .unwrap_or("")
    }

    /// Characters `from..from + len` (1-based), or `""` past the end.
    fn field(&self, from: usize, len: usize) -> &str {
        let start = from.saturating_sub(1);
        self.as_str()
            .get(start..start.saturating_add(len))
            .unwrap_or("")
    }

    /// The character at `position` (1-based), or `'\0'` past the end.
    fn at(&self, position: usize) -> char {
        self.field(position, 1).chars().next().unwrap_or('\0')
    }
}

impl<const N: usize> fmt::Debug for Code<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl Sidc {
    /// Parses and validates `s`. Spaces are ignored, as in rendering.
    ///
    /// ```
    /// use milsymbol::sidc::{Sidc, SidcError};
    ///
    /// let Sidc::Letter(l) = Sidc::parse("SFGPUCI----D")? else {
    ///     return Ok(());
    /// };
    /// assert_eq!(l.function_id(), "UCI---");
    /// let err = Sidc::parse("SFQPUCI-----").err();
    /// assert!(matches!(err, Some(SidcError::InvalidField { position: 3, .. })));
    /// # Ok::<(), SidcError>(())
    /// ```
    pub fn parse(s: &str) -> Result<Sidc, SidcError> {
        let first = s.chars().find(|&c| c != ' ').ok_or(SidcError::Empty)?;
        if first.is_ascii_digit() {
            NumericSidc::parse(s).map(Sidc::Numeric)
        } else {
            LetterSidc::parse(s).map(Sidc::Letter)
        }
    }

    /// The SIDC text (normalized).
    pub fn as_str(&self) -> &str {
        match self {
            Sidc::Numeric(n) => n.as_str(),
            Sidc::Letter(l) => l.as_str(),
        }
    }

    /// The same SIDC with another status. A letter SIDC whose status is
    /// already the requested one is returned unchanged (`-` and `P` both
    /// mean present); present is otherwise written as `P`.
    ///
    /// ```
    /// use milsymbol::domain::Status;
    /// use milsymbol::sidc::Sidc;
    ///
    /// let sidc = Sidc::parse("10031000001211000000")?;
    /// assert_eq!(sidc.with_status(Status::Damaged).as_str(), "10031030001211000000");
    /// # Ok::<(), milsymbol::sidc::SidcError>(())
    /// ```
    #[must_use]
    pub fn with_status(&self, status: Status) -> Sidc {
        match self {
            Sidc::Numeric(n) => Sidc::Numeric(n.with_status(status)),
            Sidc::Letter(l) => Sidc::Letter(l.with_status(status)),
        }
    }

    /// The same SIDC with another affiliation, written with the identity code
    /// of its context: Hostile in an exercise becomes Faker. A Faker (or
    /// Joker) is a friendly force playing the hostile role, so it is drawn
    /// with a friendly frame and a marker: `metadata().affiliation` is
    /// `Friend` and `base_affiliation` is `Hostile`.
    ///
    /// ```
    /// use milsymbol::domain::Affiliation;
    /// use milsymbol::sidc::Sidc;
    ///
    /// let numeric = Sidc::parse("10031000001211000000")?;
    /// assert_eq!(numeric.with_affiliation(Affiliation::Hostile).as_str(), "10061000001211000000");
    /// let letter = Sidc::parse("SFGPUCI----D")?;
    /// assert_eq!(letter.with_affiliation(Affiliation::Hostile).as_str(), "SHGPUCI----D");
    /// # Ok::<(), milsymbol::sidc::SidcError>(())
    /// ```
    #[must_use]
    pub fn with_affiliation(&self, affiliation: Affiliation) -> Sidc {
        match self {
            Sidc::Numeric(n) => Sidc::Numeric(n.with_affiliation(affiliation)),
            Sidc::Letter(l) => Sidc::Letter(l.with_affiliation(affiliation)),
        }
    }

    /// The same SIDC with another standard identity. Fails when the coding
    /// scheme has no code for the identity in the SIDC's context; the
    /// result's [`standard_identity`](NumericSidc::standard_identity) is
    /// always the requested one.
    ///
    /// ```
    /// use milsymbol::domain::StandardIdentity;
    /// use milsymbol::sidc::{Sidc, SidcModifyError};
    ///
    /// let sidc = Sidc::parse("10031000001211000000")?;
    /// assert_eq!(
    ///     sidc.with_standard_identity(StandardIdentity::Suspect)?.as_str(),
    ///     "10051000001211000000"
    /// );
    /// assert!(matches!(
    ///     sidc.with_standard_identity(StandardIdentity::Joker),
    ///     Err(SidcModifyError::Unrepresentable { .. })
    /// ));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_standard_identity(
        &self,
        identity: StandardIdentity,
    ) -> Result<Sidc, SidcModifyError> {
        match self {
            Sidc::Numeric(n) => n.with_standard_identity(identity).map(Sidc::Numeric),
            Sidc::Letter(l) => l.with_standard_identity(identity).map(Sidc::Letter),
        }
    }

    /// The same SIDC in another context. The identity keeps its meaning:
    /// Suspect and Hostile become Joker and Faker in an exercise, and back.
    /// The drawn frame changes with it: Joker and Faker are drawn with a
    /// friendly frame (`metadata().affiliation` is `Friend`, `base_affiliation`
    /// keeps the hostile role).
    /// Fails for a letter SIDC asked for the simulation context, or whose
    /// identity has no exercise form.
    ///
    /// ```
    /// use milsymbol::domain::Context;
    /// use milsymbol::sidc::Sidc;
    ///
    /// let letter = Sidc::parse("SHGPUCI----D")?;
    /// assert_eq!(letter.with_context(Context::Exercise)?.as_str(), "SKGPUCI----D");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn with_context(&self, context: Context) -> Result<Sidc, SidcModifyError> {
        match self {
            Sidc::Numeric(n) => Ok(Sidc::Numeric(n.with_context(context))),
            Sidc::Letter(l) => l.with_context(context).map(Sidc::Letter),
        }
    }
}

impl FromStr for Sidc {
    type Err = SidcError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Sidc::parse(s)
    }
}

impl fmt::Display for Sidc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

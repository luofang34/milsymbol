//! The packed representation of a path (feature `compact-paths`): the text is
//! produced on demand and segments are decoded from the packed bytes.

use super::parse::Parser;
use super::{PathData, PathParseError, Segment, Str, codec, packed};
use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Where the path-data text lives.
#[derive(Debug, Clone)]
pub(super) enum Source {
    Text(Str),
    /// The packed form in the generated tables (see `codec`).
    Packed(&'static [u8]),
}

impl From<Str> for Source {
    fn from(text: Str) -> Self {
        Source::Text(text)
    }
}

/// Two paths are equal when their path data is equal.
impl PartialEq for PathData {
    fn eq(&self, other: &Self) -> bool {
        match (&self.source, &other.source) {
            (Source::Text(a), Source::Text(b)) => a == b,
            (Source::Packed(a), Source::Packed(b)) if a == b => true,
            (Source::Packed(_), Source::Packed(_)) => self.source() == other.source(),
            (Source::Packed(_), Source::Text(t)) => self.matches_text(t),
            (Source::Text(t), Source::Packed(_)) => other.matches_text(t),
        }
    }
}

/// A sink that only counts the bytes written.
struct Length(usize);

impl fmt::Write for Length {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.wrapping_add(s.len());
        Ok(())
    }
}

/// A sink that succeeds while the text written is a prefix of what remains.
struct Matcher<'a>(&'a str);

impl fmt::Write for Matcher<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 = self.0.strip_prefix(s).ok_or(fmt::Error)?;
        Ok(())
    }
}

impl PathData {
    /// A path that keeps its packed form and produces text and segments on
    /// demand, so building one allocates nothing.
    pub(crate) fn from_packed(bytes: &'static [u8]) -> Self {
        PathData {
            source: Source::Packed(bytes),
            parsed: None,
        }
    }

    pub(super) fn source_cow(&self) -> Cow<'_, str> {
        match &self.source {
            Source::Text(s) => Cow::Borrowed(s),
            Source::Packed(bytes) => {
                let mut size = Length(0);
                codec::write_text(bytes, &mut size).ok();
                let mut d = String::with_capacity(size.0);
                codec::write_text(bytes, &mut d).ok();
                Cow::Owned(d)
            }
        }
    }

    pub(super) fn write_text<W: fmt::Write>(&self, out: &mut W) -> fmt::Result {
        match &self.source {
            Source::Text(s) => out.write_str(s),
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
            Source::Packed(bytes) => codec::raw_text(bytes),
        }
    }

    /// Whether the text contains `null`, which marks an invalid upstream
    /// symbol.
    pub(crate) fn contains_null(&self) -> bool {
        self.as_text().is_some_and(|t| t.contains("null"))
    }

    fn matches_text(&self, text: &str) -> bool {
        let mut rest = Matcher(text);
        self.write_text(&mut rest).is_ok() && rest.0.is_empty()
    }

    pub(super) fn parse_source(&self) -> Result<Vec<Segment>, PathParseError> {
        match &self.source {
            Source::Text(s) => Parser::new(s).run(),
            Source::Packed(bytes) => match codec::raw_text(bytes) {
                Some(text) => Parser::new(text).run(),
                None => match packed::segments(bytes) {
                    Some(segments) => Ok(segments),
                    None => Parser::new(&self.source()).run(),
                },
            },
        }
    }
}

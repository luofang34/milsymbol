//! Short inline UTF-8 strings (`<= N` bytes on the stack, `String` past `N`).

use crate::js::JsStr;
use alloc::string::String;
use core::fmt;
use core::ops::Deref;

/// A UTF-8 string stored inline up to `N` bytes, spilling to `String` only
/// when longer than `N` bytes.
#[derive(Clone)]
pub(crate) enum SmallStr<const N: usize> {
    Inline { buf: [u8; N], len: u8 },
    Heap(String),
}

impl<const N: usize> Default for SmallStr<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> SmallStr<N> {
    pub(crate) const fn new() -> Self {
        SmallStr::Inline {
            buf: [0; N],
            len: 0,
        }
    }

    /// Wraps an already-validated UTF-8 (e.g. ASCII) inline buffer.
    pub(crate) const fn from_utf8_buf(buf: [u8; N], len: u8) -> Self {
        SmallStr::Inline { buf, len }
    }

    pub(crate) fn from_str_slice(s: &str) -> Self {
        if let (Ok(len), true) = (u8::try_from(s.len()), s.len() <= N) {
            let mut buf = [0u8; N];
            if let Some(dst) = buf.get_mut(..s.len()) {
                dst.copy_from_slice(s.as_bytes());
                return SmallStr::Inline { buf, len };
            }
        }
        SmallStr::Heap(String::from(s))
    }

    pub(crate) fn from_string(s: String) -> Self {
        SmallStr::Heap(s)
    }

    pub(crate) fn as_str(&self) -> &str {
        match self {
            SmallStr::Inline { buf, len } => buf
                .get(..usize::from(*len))
                .and_then(|b| core::str::from_utf8(b).ok())
                .unwrap_or(""),
            SmallStr::Heap(s) => s.as_str(),
        }
    }

    pub(crate) fn make_ascii_uppercase(&mut self) {
        match self {
            SmallStr::Inline { buf, len } => {
                if let Some(s) = buf.get_mut(..usize::from(*len)) {
                    s.make_ascii_uppercase();
                }
            }
            SmallStr::Heap(s) => s.make_ascii_uppercase(),
        }
    }
}

impl<const N: usize> Deref for SmallStr<N> {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl<const N: usize> PartialEq for SmallStr<N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl<const N: usize> Eq for SmallStr<N> {}

impl<const N: usize> fmt::Debug for SmallStr<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// Builds the `"X-X-XXXXXX"` generic letter-SIDC key inline without allocating.
pub(crate) fn generic_letter_sidc(sidc: &JsStr<'_>) -> SmallStr<22> {
    let (a, b, c) = (sidc.substr(0, 1), sidc.substr(2, 1), sidc.substr(4, 6));
    let total = a.len() + 1 + b.len() + 1 + c.len();
    if let (Ok(len), true) = (u8::try_from(total), total <= 22) {
        let mut buf = [0u8; 22];
        let mut pos = 0usize;
        for part in [a.as_bytes(), b"-", b.as_bytes(), b"-", c.as_bytes()] {
            let next = pos + part.len();
            if let Some(dst) = buf.get_mut(pos..next) {
                dst.copy_from_slice(part);
            }
            pos = next;
        }
        return SmallStr::from_utf8_buf(buf, len);
    }
    SmallStr::from_string(alloc::format!("{a}-{b}-{c}"))
}

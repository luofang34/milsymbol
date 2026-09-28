//! Lone UTF-16 surrogates in oracle records.
//!
//! milsymbol.js takes UTF-16 substrings of a SIDC, so a SIDC containing a
//! non-BMP character can put half a surrogate pair into metadata, which
//! `JSON.stringify` writes as `\udXXX`. serde_json (like any Rust string)
//! cannot hold such a value, and the Rust port writes U+FFFD instead
//! (UPSTREAM.md, "Known differences").

/// `text` (JSON) with every escaped lone surrogate replaced by `\ufffd`, or
/// `None` if it has none.
pub(super) fn replace_lone(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let (mut i, mut last, mut found) = (0, 0, false);
    while let Some(&b) = bytes.get(i) {
        if b != b'\\' {
            i += 1;
            continue;
        }
        let Some(unit) = escape_at(bytes, i) else {
            // Any other escape: skip the backslash and the escaped byte.
            i += 2;
            continue;
        };
        let paired = if (0xD800..0xDC00).contains(&unit) {
            escape_at(bytes, i + 6).is_some_and(|next| (0xDC00..0xE000).contains(&next))
        } else {
            false
        };
        if paired {
            i += 12;
        } else if (0xD800..0xE000).contains(&unit) {
            out.push_str(text.get(last..i).unwrap_or(""));
            out.push_str("\\ufffd");
            i += 6;
            last = i;
            found = true;
        } else {
            i += 6;
        }
    }
    found.then(|| {
        out.push_str(text.get(last..).unwrap_or(""));
        out
    })
}

/// The code unit of a `\uXXXX` escape starting at byte `i`.
fn escape_at(bytes: &[u8], i: usize) -> Option<u32> {
    if bytes.get(i..i + 2)? != b"\\u" {
        return None;
    }
    let hex = std::str::from_utf8(bytes.get(i + 2..i + 6)?).ok()?;
    u32::from_str_radix(hex, 16).ok()
}

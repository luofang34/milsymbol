//! JavaScript value semantics that upstream behaviour depends on.
//!
//! milsymbol.js builds SVG attributes and path data by string concatenation
//! of IEEE doubles and compares SIDC substrings to numbers with loose
//! equality. Byte-identical output therefore needs `Number.prototype.toString`,
//! `ToNumber(string)` and UTF-16 `substr` semantics.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write as _;

/// Formats `v` exactly like JavaScript's `String(v)` for numbers.
pub fn number_to_string(v: f64) -> String {
    let mut out = String::new();
    write_number(&mut out, v);
    out
}

/// Appends `v` formatted like JavaScript's `String(v)`.
pub fn write_number(out: &mut String, v: f64) {
    if v.is_nan() {
        out.push_str("NaN");
        return;
    }
    if v == 0.0 {
        out.push('0');
        return;
    }
    if v.is_infinite() {
        out.push_str(if v > 0.0 { "Infinity" } else { "-Infinity" });
        return;
    }
    // Integers below 2^53 print as plain digits in JavaScript.
    if libm::trunc(v) == v && v.abs() < 9_007_199_254_740_992.0 {
        write!(out, "{}", v as i64).ok();
        return;
    }
    if v < 0.0 {
        out.push('-');
    }
    let (digits, n) = shortest_digits(v.abs());
    let k = digits.len() as i32;
    if k <= n && n <= 21 {
        out.push_str(&digits);
        for _ in 0..(n - k) {
            out.push('0');
        }
    } else if 0 < n && n <= 21 {
        let (int, frac) = digits.split_at(n as usize);
        out.push_str(int);
        out.push('.');
        out.push_str(frac);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        for _ in 0..(-n) {
            out.push('0');
        }
        out.push_str(&digits);
    } else {
        let (first, rest) = digits.split_at(1);
        out.push_str(first);
        if !rest.is_empty() {
            out.push('.');
            out.push_str(rest);
        }
        out.push('e');
        let e = n - 1;
        out.push(if e < 0 { '-' } else { '+' });
        // `write!` into a String cannot fail.
        write!(out, "{}", e.unsigned_abs()).ok();
    }
}

/// Shortest round-trip decimal digits of a positive finite `v`, and the
/// exponent `n` such that `v = 0.d1d2… × 10^n`.
fn shortest_digits(v: f64) -> (String, i32) {
    // Rust's `{:e}` emits the shortest representation that round-trips,
    // which is the digit string ECMAScript's Number::toString requires.
    let s = alloc::format!("{v:e}");
    let (mantissa, exp) = s.split_once('e').unwrap_or((s.as_str(), "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    (String::from(digits), exp + 1)
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{A}' | '\u{B}' | '\u{C}' | '\u{D}' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

/// JavaScript `ToNumber` applied to a string.
pub fn string_to_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_whitespace);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let bytes = t.as_bytes();
    if bytes.len() > 2 && bytes.first() == Some(&b'0') {
        let radix = match bytes.get(1) {
            Some(b'x' | b'X') => 16,
            Some(b'o' | b'O') => 8,
            Some(b'b' | b'B') => 2,
            _ => 0,
        };
        if radix != 0 {
            return parse_radix(t.get(2..).unwrap_or(""), radix);
        }
    }
    if is_decimal_literal(bytes) {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

fn parse_radix(digits: &str, radix: u32) -> f64 {
    if digits.is_empty() {
        return f64::NAN;
    }
    let mut acc = 0.0f64;
    for c in digits.chars() {
        match c.to_digit(radix) {
            Some(d) => acc = acc * f64::from(radix) + f64::from(d),
            None => return f64::NAN,
        }
    }
    acc
}

fn is_decimal_literal(b: &[u8]) -> bool {
    let mut i = 0;
    if matches!(b.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let int_start = i;
    while b.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i > int_start;
    if b.get(i) == Some(&b'.') {
        i += 1;
        let frac_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits |= i > frac_start;
    }
    if !digits {
        return false;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exp_start = i;
        while b.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exp_start {
            return false;
        }
    }
    i == b.len()
}

/// JavaScript `isNaN(s)` for a string argument.
pub fn is_nan_str(s: &str) -> bool {
    string_to_number(s).is_nan()
}

/// JavaScript `parseInt(s)` (radix 10) for the single-character strings
/// upstream passes it; returns `None` for `NaN`.
pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start_matches(is_js_whitespace);
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
    let v: i64 = digits.parse().ok()?;
    Some(if neg { -v } else { v })
}

/// JavaScript `Math.max(a, b)`: NaN-propagating, `+0` above `-0`.
pub fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { b } else { a }
    } else if a > b {
        a
    } else {
        b
    }
}

/// JavaScript `Math.min(a, b)`: NaN-propagating, `-0` below `+0`.
pub fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == b {
        if a.is_sign_negative() { a } else { b }
    } else if a < b {
        a
    } else {
        b
    }
}

/// A string viewed as UTF-16 code units, as JavaScript string methods see it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsStr {
    /// ASCII text: bytes and code units coincide.
    Ascii(String),
    /// Other text, as UTF-16 code units.
    Utf16(Vec<u16>),
}

impl Default for JsStr {
    fn default() -> Self {
        JsStr::Ascii(String::new())
    }
}

impl JsStr {
    /// Wraps `s`.
    pub fn new(s: &str) -> Self {
        if s.is_ascii() {
            JsStr::Ascii(String::from(s))
        } else {
            JsStr::Utf16(s.encode_utf16().collect())
        }
    }

    /// Length in UTF-16 code units (`String.prototype.length`).
    pub fn len(&self) -> usize {
        match self {
            JsStr::Ascii(s) => s.len(),
            JsStr::Utf16(u) => u.len(),
        }
    }

    /// `String.prototype.substr(start, len)` for non-negative arguments.
    pub fn substr(&self, start: usize, len: usize) -> String {
        let end = start.saturating_add(len).min(self.len());
        let start = start.min(end);
        match self {
            JsStr::Ascii(s) => String::from(s.get(start..end).unwrap_or("")),
            JsStr::Utf16(u) => String::from_utf16_lossy(u.get(start..end).unwrap_or(&[])),
        }
    }

    /// `String.prototype.charAt(i)`.
    pub fn char_at(&self, i: usize) -> String {
        self.substr(i, 1)
    }
}

/// Length of `s` in UTF-16 code units.
pub fn utf16_len(s: &str) -> usize {
    if s.is_ascii() {
        s.len()
    } else {
        s.encode_utf16().count()
    }
}

/// JavaScript `substr(start, len)` on a Rust string (UTF-16 semantics).
pub fn substr(s: &str, start: usize, len: usize) -> String {
    JsStr::new(s).substr(start, len)
}

#[cfg(test)]
mod tests;

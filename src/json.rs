//! Canonical JSON view of a rendered symbol, matching how milsymbol.js
//! serializes the same objects (`JSON.stringify` with keys sorted).
//!
//! Used for differential testing against the JavaScript oracle and for
//! diagnostics; not needed for rendering.

use crate::js::write_number;
use alloc::string::String;
use alloc::vec::Vec;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    /// `null`.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number (non-finite values serialize as `null`).
    Num(f64),
    /// A string.
    Str(String),
    /// An array.
    Arr(Vec<Json>),
    /// An object; keys are sorted on output.
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Serializes with keys sorted by UTF-16 code units, as the oracle does.
    pub fn to_canonical_string(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) if n.is_finite() => write_number(out, *n),
            Json::Num(_) => out.push_str("null"),
            Json::Str(s) => write_str(out, s),
            Json::Arr(v) => {
                out.push('[');
                for (i, e) in v.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    e.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                let mut sorted: Vec<&(String, Json)> = fields.iter().collect();
                sorted.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
                out.push('{');
                for (i, (k, v)) in sorted.into_iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_str(out, k);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use core::fmt::Write as _;
                write!(out, "\\u{:04x}", c as u32).ok();
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

mod fields;
mod view;
pub(crate) use view::Value;
#[cfg(test)]
mod tests;

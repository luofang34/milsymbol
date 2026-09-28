//! Canonical byte encoding of a render request.
//!
//! Every float is written as its exact bit pattern: `-0.0` and `0.0` (which
//! can render differently) are distinct keys, and a NaN matches the same
//! NaN, so key equality and hashing always agree. The options are
//! destructured without `..`, so a field added to them cannot be left out of
//! the key without a compile error.

use crate::color::ColorMode;
use crate::ir::Paint;
use crate::options::{Style, StyleColor, SymbolOptions};
use std::vec::Vec;

/// Key bytes for a lookup: on the stack for typical requests, spilling to
/// the heap for large text fields, so a cache hit does not allocate.
pub(super) struct KeyBuf {
    stack: [u8; KeyBuf::INLINE],
    len: usize,
    heap: Vec<u8>,
}

impl Default for KeyBuf {
    fn default() -> Self {
        KeyBuf {
            stack: [0; KeyBuf::INLINE],
            len: 0,
            heap: Vec::new(),
        }
    }
}

impl KeyBuf {
    const INLINE: usize = 1024;

    fn extend_from_slice(&mut self, b: &[u8]) {
        if self.heap.is_empty() {
            let end = self.len.saturating_add(b.len());
            if let Some(dst) = self.stack.get_mut(self.len..end) {
                dst.copy_from_slice(b);
                self.len = end;
                return;
            }
            self.heap
                .extend_from_slice(self.stack.get(..self.len).unwrap_or(&[]));
        }
        self.heap.extend_from_slice(b);
    }

    fn push(&mut self, b: u8) {
        self.extend_from_slice(&[b]);
    }

    pub(super) fn as_slice(&self) -> &[u8] {
        if self.heap.is_empty() {
            self.stack.get(..self.len).unwrap_or(&[])
        } else {
            &self.heap
        }
    }
}

/// Appends length-prefixed bytes so adjacent fields cannot run together.
fn put_bytes(out: &mut KeyBuf, b: &[u8]) {
    out.extend_from_slice(&(b.len() as u64).to_le_bytes());
    out.extend_from_slice(b);
}

fn put_f64(out: &mut KeyBuf, v: f64) {
    out.extend_from_slice(&v.to_bits().to_le_bytes());
}

fn put_opt_f64(out: &mut KeyBuf, v: Option<f64>) {
    match v {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            put_f64(out, v);
        }
    }
}

fn put_opt_str(out: &mut KeyBuf, v: Option<&str>) {
    match v {
        None => out.push(0),
        Some(s) => {
            out.push(1);
            put_bytes(out, s.as_bytes());
        }
    }
}

fn put_mode(out: &mut KeyBuf, m: &ColorMode) {
    for v in m.values() {
        match v {
            None => out.push(0),
            Some(Paint::None) => out.push(1),
            Some(Paint::Color(c)) => {
                out.push(2);
                put_bytes(out, c.as_bytes());
            }
        }
    }
}

fn put_style_color(out: &mut KeyBuf, c: &StyleColor) {
    match c {
        StyleColor::Str(s) => {
            out.push(0);
            put_bytes(out, s.as_bytes());
        }
        StyleColor::PerAffiliation(m) => {
            out.push(1);
            put_mode(out, m);
        }
    }
}

/// Writes the canonical byte encoding of a render request.
pub(super) fn write_key(k: &mut KeyBuf, sidc: &str, o: &SymbolOptions) {
    let SymbolOptions {
        text,
        direction,
        speed_leader,
        stack,
        country_flag,
        full_frame_flag,
        signature,
        style,
    } = o;
    put_bytes(k, sidc.as_bytes());
    k.extend_from_slice(&(text.len() as u64).to_le_bytes());
    for (name, value) in text {
        put_bytes(k, name.as_bytes());
        put_bytes(k, value.as_bytes());
    }
    put_opt_f64(k, *direction);
    put_f64(k, *speed_leader);
    put_opt_f64(k, *stack);
    put_opt_str(k, country_flag.as_deref());
    put_opt_str(k, signature.as_deref());
    k.push(match full_frame_flag {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    });
    write_style(k, style);
}

fn write_style(k: &mut KeyBuf, st: &Style) {
    let Style {
        alternate_medal,
        civilian_color,
        color_mode,
        fill,
        fill_color,
        fill_opacity,
        font_family,
        frame,
        frame_color,
        hq_staff_length,
        icon,
        icon_color,
        info_background,
        info_background_frame,
        info_color,
        info_fields,
        info_outline_color,
        info_outline_width,
        info_size,
        mono_color,
        outline_color,
        outline_width,
        padding,
        simple_status_modifier,
        size,
        square,
        standard,
        stroke_width,
        style_fill,
    } = st;
    for v in [
        fill_opacity,
        hq_staff_length,
        info_size,
        outline_width,
        padding,
        size,
        stroke_width,
    ] {
        put_f64(k, *v);
    }
    put_opt_f64(k, *info_outline_width);
    let flags = [
        alternate_medal,
        civilian_color,
        fill,
        frame,
        icon,
        info_fields,
        simple_status_modifier,
        square,
        style_fill,
    ];
    k.extend_from_slice(&flags.map(|f| u8::from(*f)));
    for s in [fill_color, font_family, info_outline_color, mono_color] {
        put_bytes(k, s.as_bytes());
    }
    k.push(match standard {
        None => 0,
        Some(crate::Standard::Mil2525) => 1,
        Some(crate::Standard::App6) => 2,
    });
    for c in [
        color_mode,
        frame_color,
        icon_color,
        info_background,
        info_background_frame,
        info_color,
        outline_color,
    ] {
        put_style_color(k, c);
    }
}

#[cfg(test)]
mod tests;

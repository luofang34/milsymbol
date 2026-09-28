//! A memoizing wrapper around [`Renderer`] (requires `std`).
//!
//! Rendering is deterministic for a renderer, SIDC and options, so results
//! can be shared. Entries are keyed by a canonical encoding of the SIDC and
//! options in which every float is its exact bit pattern: `-0.0` and `0.0`
//! (which can render differently) are distinct keys, and a NaN matches the
//! same NaN, so key equality and hashing always agree.

use crate::color::ColorMode;
use crate::error::RenderError;
use crate::ir::Paint;
use crate::options::{StyleColor, SymbolOptions};
use crate::renderer::Renderer;
use crate::symbol::Symbol;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

/// A renderer with a bounded cache of rendered symbols.
///
/// When the cache holds `capacity` entries it is emptied before inserting;
/// a capacity of zero disables caching.
pub struct CachedRenderer {
    renderer: Renderer,
    capacity: usize,
    entries: Mutex<HashMap<Vec<u8>, Arc<Symbol>>>,
}

impl core::fmt::Debug for CachedRenderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CachedRenderer")
            .field("renderer", &self.renderer)
            .field("capacity", &self.capacity)
            .finish()
    }
}

impl CachedRenderer {
    /// Wraps `renderer` with room for `capacity` symbols.
    pub fn new(renderer: Renderer, capacity: usize) -> Self {
        CachedRenderer {
            renderer,
            capacity,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// The wrapped renderer.
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Number of cached symbols.
    pub fn len(&self) -> usize {
        self.entries.lock().map_or(0, |e| e.len())
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Renders `sidc` with `options`, reusing an earlier identical render.
    pub fn render(&self, sidc: &str, options: &SymbolOptions) -> Result<Arc<Symbol>, RenderError> {
        if self.capacity == 0 {
            return Ok(Arc::new(self.renderer.render(sidc, options.clone())?));
        }
        let mut key = KeyBuf::default();
        write_key(&mut key, sidc, options);
        let key = key.as_slice();
        if let Some(hit) = self.entries.lock().ok().and_then(|e| e.get(key).cloned()) {
            return Ok(hit);
        }
        // Rendering happens outside the lock so concurrent misses do not serialize.
        let symbol = Arc::new(self.renderer.render(sidc, options.clone())?);
        let evicted = match self.entries.lock() {
            Ok(mut entries) => {
                let old = if entries.len() >= self.capacity {
                    core::mem::take(&mut *entries)
                } else {
                    HashMap::new()
                };
                entries.insert(key.to_vec(), Arc::clone(&symbol));
                old
            }
            Err(_) => HashMap::new(),
        };
        // Dropping the evicted symbols happens after the lock is released.
        drop(evicted);
        Ok(symbol)
    }
}

/// Key bytes for a lookup: on the stack for typical requests, spilling to
/// the heap for large text fields, so a cache hit does not allocate.
struct KeyBuf {
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

    fn as_slice(&self) -> &[u8] {
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
fn write_key(k: &mut KeyBuf, sidc: &str, o: &SymbolOptions) {
    put_bytes(k, sidc.as_bytes());
    k.extend_from_slice(&(o.text.len() as u64).to_le_bytes());
    for (name, value) in &o.text {
        put_bytes(k, name.as_bytes());
        put_bytes(k, value.as_bytes());
    }
    put_opt_f64(k, o.direction);
    put_f64(k, o.speed_leader);
    put_opt_f64(k, o.stack);
    put_opt_str(k, o.country_flag.as_deref());
    put_opt_str(k, o.signature.as_deref());
    k.push(match o.full_frame_flag {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    });
    let st = &o.style;
    for v in [
        st.fill_opacity,
        st.hq_staff_length,
        st.info_size,
        st.outline_width,
        st.padding,
        st.size,
        st.stroke_width,
    ] {
        put_f64(k, v);
    }
    put_opt_f64(k, st.info_outline_width);
    let flags = [
        st.alternate_medal,
        st.civilian_color,
        st.fill,
        st.frame,
        st.icon,
        st.info_fields,
        st.simple_status_modifier,
        st.square,
        st.style_fill,
    ];
    k.extend_from_slice(&flags.map(u8::from));
    for s in [
        &st.fill_color,
        &st.font_family,
        &st.info_outline_color,
        &st.mono_color,
    ] {
        put_bytes(k, s.as_bytes());
    }
    k.push(match st.standard {
        None => 0,
        Some(crate::Standard::Mil2525) => 1,
        Some(crate::Standard::App6) => 2,
    });
    for c in [
        &st.color_mode,
        &st.frame_color,
        &st.icon_color,
        &st.info_background,
        &st.info_background_frame,
        &st.info_color,
        &st.outline_color,
    ] {
        put_style_color(k, c);
    }
}

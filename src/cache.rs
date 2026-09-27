//! A memoizing wrapper around [`Renderer`] (requires `std`).
//!
//! Rendering is deterministic for a renderer, SIDC and options, so results
//! can be shared. Entries are keyed by the SIDC and the full options value;
//! options containing NaN never compare equal and are simply not reused.

use crate::color::ColorMode;
use crate::error::RenderError;
use crate::ir::Paint;
use crate::options::StyleColor;
use crate::options::SymbolOptions;
use crate::renderer::Renderer;
use crate::symbol::Symbol;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::string::String;
use std::sync::{Arc, Mutex};

/// A renderer with a bounded cache of rendered symbols.
///
/// When the cache holds `capacity` entries it is cleared before inserting.
pub struct CachedRenderer {
    renderer: Renderer,
    capacity: usize,
    entries: Mutex<HashMap<Key, Arc<Symbol>>>,
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
        let key = Key {
            sidc: String::from(sidc),
            options: options.clone(),
        };
        if let Some(hit) = self.entries.lock().ok().and_then(|e| e.get(&key).cloned()) {
            return Ok(hit);
        }
        // Rendering happens outside the lock so concurrent misses do not serialize.
        let symbol = Arc::new(self.renderer.render(sidc, options.clone())?);
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= self.capacity {
                entries.clear();
            }
            entries.insert(key, Arc::clone(&symbol));
        }
        Ok(symbol)
    }
}

/// Cache key: SIDC plus options, hashed structurally.
struct Key {
    sidc: String,
    options: SymbolOptions,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.sidc == other.sidc && self.options == other.options
    }
}

// Options holding NaN are unequal to themselves; such keys only ever miss.
impl Eq for Key {}

fn hash_f64<H: Hasher>(v: f64, h: &mut H) {
    v.to_bits().hash(h);
}

fn hash_mode<H: Hasher>(m: &ColorMode, h: &mut H) {
    for v in m.values() {
        match v {
            None => 0u8.hash(h),
            Some(Paint::None) => 1u8.hash(h),
            Some(Paint::Color(c)) => {
                2u8.hash(h);
                c.hash(h);
            }
        }
    }
}

fn hash_style_color<H: Hasher>(c: &StyleColor, h: &mut H) {
    match c {
        StyleColor::Str(s) => s.hash(h),
        StyleColor::PerAffiliation(m) => hash_mode(m, h),
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, h: &mut H) {
        let o = &self.options;
        self.sidc.hash(h);
        o.text.hash(h);
        o.direction.map(f64::to_bits).hash(h);
        hash_f64(o.speed_leader, h);
        o.stack.map(f64::to_bits).hash(h);
        (&o.country_flag, o.full_frame_flag, &o.signature).hash(h);
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
            hash_f64(v, h);
        }
        st.info_outline_width.map(f64::to_bits).hash(h);
        let flags = [
            st.alternate_medal,
            st.civilian_color,
            st.fill,
            st.frame,
            st.icon,
            st.info_fields,
        ];
        (flags, st.simple_status_modifier, st.square, st.style_fill).hash(h);
        (
            &st.fill_color,
            &st.font_family,
            &st.info_outline_color,
            &st.mono_color,
            &st.standard,
        )
            .hash(h);
        for c in [
            &st.color_mode,
            &st.frame_color,
            &st.icon_color,
            &st.info_background,
            &st.info_background_frame,
            &st.info_color,
            &st.outline_color,
        ] {
            hash_style_color(c, h);
        }
    }
}

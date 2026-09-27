//! A memoizing wrapper around [`Renderer`] (requires `std`).
//!
//! Rendering is deterministic for a renderer, SIDC and options, so results
//! can be shared. The cache key is the SIDC plus the canonical JSON of the
//! options, so any option difference yields a distinct entry.

use crate::error::RenderError;
use crate::options::SymbolOptions;
use crate::renderer::Renderer;
use crate::symbol::Symbol;
use std::collections::HashMap;
use std::string::String;
use std::sync::{Arc, Mutex};

/// A renderer with a bounded cache of rendered symbols.
///
/// When the cache holds `capacity` entries it is cleared before inserting.
pub struct CachedRenderer {
    renderer: Renderer,
    capacity: usize,
    entries: Mutex<HashMap<(String, String), Arc<Symbol>>>,
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
        let key = (
            String::from(sidc),
            crate::json::options(sidc, options).to_canonical_string(),
        );
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

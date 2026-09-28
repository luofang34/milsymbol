//! A memoizing wrapper around [`Renderer`] (requires `std`).
//!
//! Rendering is deterministic for a renderer, SIDC and options, so results
//! can be shared. Entries are keyed by a canonical encoding of the SIDC and
//! every option, with floats compared by bit pattern (`-0.0` and `0.0` are
//! different requests).

use crate::error::RenderError;
use crate::options::SymbolOptions;
use crate::renderer::Renderer;
use crate::symbol::Symbol;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::vec::Vec;

mod key;
use key::{KeyBuf, write_key};

/// A renderer with a bounded cache of rendered symbols.
///
/// When the cache is full, a new entry replaces one that has not been used
/// since the eviction scan last passed it (the clock algorithm, an
/// approximation of least-recently-used). A capacity of zero disables
/// caching.
///
/// ```
/// use milsymbol::{Renderer, cache::CachedRenderer, options::SymbolOptions};
/// use std::sync::Arc;
///
/// let cache = CachedRenderer::new(Renderer::default(), 1024).with_prepared_paths();
/// let options = SymbolOptions::default();
/// let a = cache.render("10031000001211000000", &options)?;
/// let b = cache.render("10031000001211000000", &options)?;
/// assert!(Arc::ptr_eq(&a, &b)); // the second call is a cache hit
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
pub struct CachedRenderer {
    renderer: Renderer,
    capacity: usize,
    prepare_paths: bool,
    entries: Mutex<Clock>,
}

impl core::fmt::Debug for CachedRenderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CachedRenderer")
            .field("renderer", &self.renderer)
            .field("capacity", &self.capacity)
            .field("prepare_paths", &self.prepare_paths)
            .finish()
    }
}

/// One cached symbol.
struct Slot {
    key: Arc<[u8]>,
    symbol: Arc<Symbol>,
    /// Used since the clock hand last passed.
    referenced: bool,
}

/// Cached symbols in a ring, with an index by key.
#[derive(Default)]
struct Clock {
    slots: Vec<Slot>,
    index: HashMap<Arc<[u8]>, usize>,
    hand: usize,
}

impl Clock {
    fn get(&mut self, key: &[u8]) -> Option<Arc<Symbol>> {
        let slot = self.slots.get_mut(*self.index.get(key)?)?;
        slot.referenced = true;
        Some(Arc::clone(&slot.symbol))
    }

    /// Inserts `symbol` (unless `key` is already present, whose symbol is
    /// returned instead) and returns the evicted symbol, if any, so it can be
    /// dropped outside the lock.
    fn insert(
        &mut self,
        key: &[u8],
        symbol: &Arc<Symbol>,
        capacity: usize,
    ) -> Result<Option<Arc<Symbol>>, Arc<Symbol>> {
        if let Some(existing) = self.get(key) {
            return Err(existing);
        }
        let key: Arc<[u8]> = Arc::from(key);
        let slot = Slot {
            key: Arc::clone(&key),
            symbol: Arc::clone(symbol),
            referenced: false,
        };
        if self.slots.len() < capacity {
            self.index.insert(key, self.slots.len());
            self.slots.push(slot);
            return Ok(None);
        }
        let victim = self.advance_to_victim();
        let Some(old) = self.slots.get_mut(victim) else {
            return Ok(None);
        };
        let old = core::mem::replace(old, slot);
        self.index.remove(&old.key);
        self.index.insert(key, victim);
        self.hand = (victim + 1) % self.slots.len().max(1);
        Ok(Some(old.symbol))
    }

    /// Moves the hand to the first slot not used since it last passed,
    /// clearing the reference bits it passes over.
    fn advance_to_victim(&mut self) -> usize {
        let n = self.slots.len().max(1);
        loop {
            let hand = self.hand % n;
            match self.slots.get_mut(hand) {
                Some(slot) if slot.referenced => {
                    slot.referenced = false;
                    self.hand = (hand + 1) % n;
                }
                _ => return hand,
            }
        }
    }
}

impl CachedRenderer {
    /// Wraps `renderer` with room for `capacity` symbols.
    pub fn new(renderer: Renderer, capacity: usize) -> Self {
        CachedRenderer {
            renderer,
            capacity,
            prepare_paths: false,
            entries: Mutex::new(Clock::default()),
        }
    }

    /// Parses every path's segments before a symbol is cached, so backends
    /// that read [`PathData::segments`](crate::ir::PathData::segments) get
    /// them without parsing (a shared `Arc<Symbol>` cannot be prepared
    /// afterwards). A path that fails to parse is left as is; `segments()`
    /// then reports the error.
    ///
    /// Enabling this mode clears unprepared cache entries; symbols already
    /// returned to callers remain usable. Repeated calls preserve the cache.
    pub fn with_prepared_paths(mut self) -> Self {
        if !self.prepare_paths {
            self.entries = Mutex::new(Clock::default());
        }
        self.prepare_paths = true;
        self
    }

    /// The wrapped renderer.
    pub fn renderer(&self) -> &Renderer {
        &self.renderer
    }

    /// Number of cached symbols.
    pub fn len(&self) -> usize {
        self.entries.lock().map_or(0, |e| e.slots.len())
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Renders `sidc` with `options`, reusing an earlier identical render.
    pub fn render(&self, sidc: &str, options: &SymbolOptions) -> Result<Arc<Symbol>, RenderError> {
        if self.capacity == 0 {
            return Ok(Arc::new(self.render_uncached(sidc, options)?));
        }
        let mut key = KeyBuf::default();
        write_key(&mut key, sidc, options);
        let key = key.as_slice();
        if let Some(hit) = self.entries.lock().ok().and_then(|mut e| e.get(key)) {
            return Ok(hit);
        }
        // Rendering happens outside the lock so concurrent misses do not serialize.
        let symbol = Arc::new(self.render_uncached(sidc, options)?);
        let inserted = match self.entries.lock() {
            Ok(mut entries) => entries.insert(key, &symbol, self.capacity),
            Err(_) => Ok(None),
        };
        match inserted {
            // Dropping the evicted symbol happens after the lock is released.
            Ok(evicted) => {
                drop(evicted);
                Ok(symbol)
            }
            // Another render populated this key while the lock was released.
            Err(existing) => Ok(existing),
        }
    }

    fn render_uncached(&self, sidc: &str, options: &SymbolOptions) -> Result<Symbol, RenderError> {
        let mut symbol = self.renderer.render(sidc, options.clone())?;
        if self.prepare_paths {
            symbol.cache_path_segments().ok();
        }
        Ok(symbol)
    }
}

#[cfg(test)]
mod tests;

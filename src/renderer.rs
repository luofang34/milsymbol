//! The renderer: explicit configuration and extensions, shared by all
//! symbols it draws.

use crate::color::ColorMode;
use crate::compose::{self, BuiltinPart, SymbolPart};
use crate::config::{DashArrays, RendererConfig, Standard};
use crate::error::RenderError;
use crate::options::SymbolOptions;
use crate::registry::{IconExtension, PartSlot, Registry};
use crate::symbol::Symbol;
use alloc::boxed::Box;
use alloc::string::String;

/// Draws symbols with a fixed configuration.
///
/// A renderer is immutable while drawing, so one instance can serve many
/// threads (`Renderer: Send + Sync`). Everything milsymbol.js keeps in
/// process-global state — standard, dash arrays, HQ staff length, colour
/// modes, symbol parts, icon and label extensions — lives here instead.
pub struct Renderer {
    config: RendererConfig,
    registry: Registry,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::new(RendererConfig::default())
    }
}

impl core::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Renderer")
            .field("config", &self.config)
            .field("parts", &self.registry.parts.len())
            .field("icon_extensions", &self.registry.icons.len())
            .finish()
    }
}

impl Renderer {
    /// A renderer with the given configuration and the built-in pipeline.
    pub fn new(config: RendererConfig) -> Self {
        Renderer {
            config,
            registry: Registry::default(),
        }
    }

    /// The configuration.
    pub fn config(&self) -> &RendererConfig {
        &self.config
    }

    /// Mutable configuration.
    pub fn config_mut(&mut self) -> &mut RendererConfig {
        &mut self.config
    }

    /// Sets the default standard (upstream `ms.setStandard`).
    pub fn with_standard(mut self, standard: Standard) -> Self {
        self.config.standard = standard;
        self
    }

    /// Sets the dash arrays (upstream `ms.setDashArrays`).
    pub fn with_dash_arrays(mut self, dash: DashArrays) -> Self {
        self.config.dash_arrays = dash;
        self
    }

    /// Sets the default HQ staff length (upstream `ms.setHqStaffLength`).
    pub fn with_hq_staff_length(mut self, len: f64) -> Self {
        self.config.hq_staff_length = len;
        self
    }

    /// Registers or replaces a colour mode (upstream `ms.setColorMode`).
    pub fn with_color_mode(mut self, name: impl Into<String>, mode: ColorMode) -> Self {
        self.config.color_modes.insert(name.into(), mode);
        self
    }

    /// Appends a symbol part to the pipeline (upstream `ms.addSymbolPart`).
    pub fn with_symbol_part(mut self, part: impl SymbolPart + 'static) -> Self {
        self.registry.parts.push(PartSlot::Custom(Box::new(part)));
        self
    }

    /// Draws the icon octagon on every symbol (upstream `ms.showOctagon`).
    pub fn with_octagon(mut self) -> Self {
        self.registry
            .parts
            .push(PartSlot::Builtin(BuiltinPart::Octagon));
        self
    }

    /// Replaces the pipeline with the given built-in parts, in order.
    pub fn with_builtin_parts(mut self, parts: &[BuiltinPart]) -> Self {
        self.registry.parts = parts.iter().map(|&p| PartSlot::Builtin(p)).collect();
        self
    }

    /// Registers an icon extension (upstream `ms.addIcons`).
    pub fn with_icons(mut self, ext: impl IconExtension + 'static) -> Self {
        self.registry.add_icons(Box::new(ext));
        self
    }

    /// Starts a symbol.
    pub fn symbol<'r>(&'r self, sidc: &str) -> SymbolBuilder<'r> {
        SymbolBuilder {
            renderer: self,
            sidc: String::from(sidc),
            options: SymbolOptions::default(),
        }
    }

    /// Renders a symbol with the given options.
    pub fn render(&self, sidc: &str, mut options: SymbolOptions) -> Result<Symbol, RenderError> {
        let c = compose::compose(sidc, &mut options, &self.config, &self.registry)?;
        Ok(Symbol::from_composition(c, options))
    }
}

/// Builder returned by [`Renderer::symbol`].
#[derive(Debug)]
pub struct SymbolBuilder<'r> {
    renderer: &'r Renderer,
    sidc: String,
    options: SymbolOptions,
}

impl SymbolBuilder<'_> {
    /// Replaces all options.
    pub fn options(mut self, options: SymbolOptions) -> Self {
        self.options = options;
        self
    }

    /// Edits options in place.
    pub fn with(mut self, f: impl FnOnce(&mut SymbolOptions)) -> Self {
        f(&mut self.options);
        self
    }

    /// Sets a text amplifier (see [`crate::options::field`]).
    pub fn text(mut self, key: &str, value: impl Into<String>) -> Self {
        self.options.set_text(key, value);
        self
    }

    /// Sets the symbol size.
    pub fn size(mut self, size: f64) -> Self {
        self.options.style.size = size;
        self
    }

    /// Renders the symbol.
    pub fn render(self) -> Result<Symbol, RenderError> {
        self.renderer.render(&self.sidc, self.options)
    }
}

//! The renderer: explicit configuration and extensions, shared by all
//! symbols it draws.

use crate::builder::{SidcInput, SymbolBuilder};
use crate::color::ColorMode;
use crate::compose::{self, BuiltinPart, SymbolPart};
use crate::config::{DashArrays, RendererConfig, Standard};
use crate::error::RenderError;
use crate::options::SymbolOptions;
use crate::registry::{IconExtension, PartSlot, Registry};
use crate::sidc::{Sidc, SidcCheckError};
use crate::symbol::Symbol;
use alloc::boxed::Box;
use alloc::string::String;
// Targets without pointer-width atomics (thumbv6m) have no `Arc`; they are
// single-core, and `Renderer` is then `!Send + !Sync`.
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Shared;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc as Shared;

/// Draws symbols with a fixed configuration.
///
/// A renderer is immutable once built, and cloning it is cheap (clones share
/// the configuration), so one instance can serve many threads
/// (`Renderer: Send + Sync`, except on targets without atomic pointers such
/// as thumbv6m). Everything milsymbol.js keeps in process-global
/// state — standard, dash arrays, HQ staff length, colour modes, symbol
/// parts, icon and label extensions — is set through [`RendererBuilder`].
///
/// ```
/// use milsymbol::{Renderer, Standard};
///
/// // Configuration that milsymbol.js keeps globally belongs to a renderer.
/// let app6 = Renderer::builder().standard(Standard::App6).build();
/// let symbol = app6.symbol("SFGPUCI-----").size(60.0).render()?;
/// assert!(!symbol.metadata().std2525);
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
#[derive(Clone)]
pub struct Renderer {
    inner: Shared<Inner>,
}

struct Inner {
    config: RendererConfig,
    registry: Registry,
}

impl Default for Renderer {
    fn default() -> Self {
        Renderer::builder().build()
    }
}

impl core::fmt::Debug for Renderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Renderer")
            .field("config", &self.inner.config)
            .field("parts", &self.inner.registry.parts.len())
            .field("icon_extensions", &self.inner.registry.icons.len())
            .finish()
    }
}

impl Renderer {
    /// Starts configuring a renderer.
    pub fn builder() -> RendererBuilder {
        RendererBuilder {
            config: RendererConfig::default(),
            registry: Registry::default(),
        }
    }

    /// A renderer with the given configuration and the built-in pipeline.
    pub fn new(config: RendererConfig) -> Self {
        Renderer::builder().config(config).build()
    }

    /// The configuration.
    pub fn config(&self) -> &RendererConfig {
        &self.inner.config
    }

    /// Starts a symbol. `sidc` can be text (`&str`, `String`) or an already
    /// parsed [`Sidc`].
    ///
    /// ```
    /// use milsymbol::{Renderer, options::TextField};
    ///
    /// let symbol = Renderer::default()
    ///     .symbol("10031000161211000000")
    ///     .text(TextField::UniqueDesignation, "A/1-66")
    ///     .size(40.0)
    ///     .render()?;
    /// assert!(symbol.to_svg().starts_with("<svg"));
    /// # Ok::<(), milsymbol::RenderError>(())
    /// ```
    pub fn symbol<'a>(&self, sidc: impl Into<SidcInput<'a>>) -> SymbolBuilder<'a> {
        SymbolBuilder::new(self.clone(), sidc.into())
    }

    /// Checks that `sidc` is well formed ([`Sidc::parse`]) and that this
    /// renderer, with its extensions, recognises every part of it.
    ///
    /// ```
    /// use milsymbol::{Renderer, sidc::SidcCheckError};
    ///
    /// let r = Renderer::default();
    /// assert!(r.check_sidc("10031000161211000000").is_ok());
    /// // Well formed, but no built-in icon has entity 999999.
    /// assert!(matches!(
    ///     r.check_sidc("10031000009999990000"),
    ///     Err(SidcCheckError::Unsupported { .. })
    /// ));
    /// ```
    pub fn check_sidc(&self, sidc: &str) -> Result<Sidc, SidcCheckError> {
        let parsed = Sidc::parse(sidc)?;
        match self.render_checked(parsed.as_str(), SymbolOptions::default(), true) {
            Ok(_) => Ok(parsed),
            Err(RenderError::MalformedSidc(e)) => Err(SidcCheckError::Malformed(e)),
            Err(RenderError::UnsupportedSidc { issues }) => {
                Err(SidcCheckError::Unsupported { issues })
            }
            Err(e) => Err(SidcCheckError::Render(e)),
        }
    }

    /// The one place `strict` is defined: every builder, cached or not, ends
    /// here.
    pub(crate) fn render_checked(
        &self,
        sidc: &str,
        options: SymbolOptions,
        strict: bool,
    ) -> Result<Symbol, RenderError> {
        if strict {
            Sidc::parse(sidc).map_err(RenderError::MalformedSidc)?;
        }
        let symbol = self.render(sidc, options)?;
        if strict {
            let issues = symbol.sidc_issues();
            if !issues.is_empty() {
                return Err(RenderError::UnsupportedSidc { issues });
            }
        }
        Ok(symbol)
    }

    /// Renders a symbol from complete options; a shortcut for
    /// [`Renderer::symbol`] with [`SymbolBuilder::options`].
    pub fn render(&self, sidc: &str, mut options: SymbolOptions) -> Result<Symbol, RenderError> {
        let c = compose::compose(sidc, &mut options, &self.inner.config, &self.inner.registry)?;
        Ok(Symbol::from_composition(c, options))
    }
}

/// Configures a [`Renderer`].
///
/// Calls apply in order: [`RendererBuilder::pipeline`] replaces the symbol
/// parts chosen so far, while [`RendererBuilder::symbol_part`] and
/// [`RendererBuilder::octagon`] append to them.
///
/// ```
/// use milsymbol::{BuiltinPart, Renderer};
///
/// // Only the frame and the icon.
/// let renderer = Renderer::builder()
///     .pipeline(&[BuiltinPart::BaseGeometry, BuiltinPart::Icon])
///     .build();
/// # let _ = renderer;
/// ```
#[must_use = "a builder does nothing until it is built or rendered"]
pub struct RendererBuilder {
    config: RendererConfig,
    registry: Registry,
}

impl core::fmt::Debug for RendererBuilder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RendererBuilder")
            .field("config", &self.config)
            .field("parts", &self.registry.parts.len())
            .finish()
    }
}

impl RendererBuilder {
    /// Replaces the whole configuration.
    pub fn config(mut self, config: RendererConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the default standard (upstream `ms.setStandard`).
    pub fn standard(mut self, standard: Standard) -> Self {
        self.config.standard = standard;
        self
    }

    /// Selects the JavaScript engine build whose output is reproduced
    /// bit for bit (see [`ReferencePlatform`](crate::ReferencePlatform)).
    pub fn reference_platform(mut self, platform: crate::ReferencePlatform) -> Self {
        self.config.reference_platform = platform;
        self
    }

    /// Sets the dash arrays (upstream `ms.setDashArrays`).
    pub fn dash_arrays(mut self, dash: DashArrays) -> Self {
        self.config.dash_arrays = dash;
        self
    }

    /// Sets the default HQ staff length (upstream `ms.setHqStaffLength`).
    pub fn hq_staff_length(mut self, len: f64) -> Self {
        self.config.hq_staff_length = len;
        self
    }

    /// Registers or replaces a colour mode (upstream `ms.setColorMode`).
    pub fn color_mode(mut self, name: impl Into<String>, mode: ColorMode) -> Self {
        self.config.color_modes.insert(name.into(), mode);
        self
    }

    /// Replaces the symbol parts with the given built-in parts, in order.
    pub fn pipeline(mut self, parts: &[BuiltinPart]) -> Self {
        self.registry.parts = parts.iter().map(|&p| PartSlot::Builtin(p)).collect();
        self
    }

    /// Appends a symbol part (upstream `ms.addSymbolPart`).
    pub fn symbol_part(mut self, part: impl SymbolPart + 'static) -> Self {
        self.registry.parts.push(PartSlot::Custom(Box::new(part)));
        self
    }

    /// Appends the icon octagon, drawn on every symbol (upstream
    /// `ms.showOctagon`).
    pub fn octagon(mut self) -> Self {
        self.registry
            .parts
            .push(PartSlot::Builtin(BuiltinPart::Octagon));
        self
    }

    /// Registers an icon extension (upstream `ms.addIcons`).
    pub fn icons(mut self, ext: impl IconExtension + 'static) -> Self {
        self.registry.add_icons(Box::new(ext));
        self
    }

    /// Finishes the renderer.
    pub fn build(self) -> Renderer {
        Renderer {
            inner: Shared::new(Inner {
                config: self.config,
                registry: self.registry,
            }),
        }
    }
}

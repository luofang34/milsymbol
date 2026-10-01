//! Building a symbol request: the SIDC plus options.

use crate::error::RenderError;
use crate::options::SymbolOptions;
use crate::renderer::Renderer;
use crate::sidc::Sidc;
use crate::symbol::Symbol;
use alloc::borrow::Cow;
use alloc::string::String;

/// A SIDC as the input of [`Renderer::symbol`]: text, or one already parsed.
///
/// Malformed text is not an error here; milsymbol.js draws it with a `?`
/// icon and reports it through [`Symbol::validity`]. Use
/// [`SymbolBuilder::strict`] to fail instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidcInput<'a>(Cow<'a, str>);

impl SidcInput<'_> {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'a> From<&'a str> for SidcInput<'a> {
    fn from(s: &'a str) -> Self {
        SidcInput(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for SidcInput<'a> {
    fn from(s: &'a String) -> Self {
        SidcInput(Cow::Borrowed(s))
    }
}

impl From<String> for SidcInput<'_> {
    fn from(s: String) -> Self {
        SidcInput(Cow::Owned(s))
    }
}

impl<'a> From<&'a Sidc> for SidcInput<'a> {
    fn from(s: &'a Sidc) -> Self {
        SidcInput(Cow::Borrowed(s.as_str()))
    }
}

/// The option setters both builders share.
macro_rules! option_setters {
    () => {
        /// Replaces all options.
        pub fn options(mut self, options: $crate::options::SymbolOptions) -> Self {
            self.options = options;
            self
        }

        /// Edits the options in place, for any option without its own setter.
        pub fn with(mut self, f: impl FnOnce(&mut $crate::options::SymbolOptions)) -> Self {
            f(&mut self.options);
            self
        }

        /// Sets a text amplifier.
        pub fn text(
            mut self,
            field: impl Into<$crate::options::TextField>,
            value: impl Into<alloc::string::String>,
        ) -> Self {
            self.options.set_text(field, value);
            self
        }

        /// Sets the symbol size in pixels.
        pub fn size(mut self, size: f64) -> Self {
            self.options.style.size = size;
            self
        }

        /// Sets the direction of movement in degrees.
        pub fn direction(mut self, degrees: f64) -> Self {
            self.options.direction = Some(degrees);
            self
        }

        /// Draws a speed leader of this length in pixels instead of the
        /// direction arrow.
        pub fn speed_leader(mut self, length: f64) -> Self {
            self.options.speed_leader = Some(length);
            self
        }

        /// Draws this many stacked frames behind the symbol.
        pub fn stack(mut self, count: f64) -> Self {
            self.options.stack = Some(count);
            self
        }

        /// Draws all text in this font family, including the text inside
        /// built-in icons (see [`SymbolOptions::set_font`](
        /// $crate::options::SymbolOptions::set_font)). Without it, icons keep
        /// their template font, as milsymbol.js draws them.
        pub fn font(mut self, family: impl Into<$crate::ir::Str>) -> Self {
            self.options.set_font(family);
            self
        }

        /// Draws the symbol under this standard instead of the renderer's.
        pub fn standard(mut self, standard: $crate::Standard) -> Self {
            self.options.style.standard = Some(standard);
            self
        }

        /// Selects the fill colour mode.
        pub fn color_mode(mut self, mode: impl Into<$crate::options::ColorModeChoice>) -> Self {
            self.options.style.color_mode = mode.into();
            self
        }

        /// Draws the symbol in one colour.
        pub fn mono_color(mut self, color: $crate::options::Color) -> Self {
            self.options.style.mono_color = Some(color);
            self
        }
    };
}
#[cfg(feature = "std")]
pub(crate) use option_setters;

/// Builds and renders one symbol; returned by [`Renderer::symbol`].
///
/// The builder holds its own handle on the renderer, so it can be stored and
/// passed around independently of the renderer it came from.
#[derive(Debug)]
#[must_use = "a builder does nothing until it is built or rendered"]
pub struct SymbolBuilder<'a> {
    renderer: Renderer,
    sidc: SidcInput<'a>,
    options: SymbolOptions,
    strict: bool,
}

impl<'a> SymbolBuilder<'a> {
    pub(crate) fn new(renderer: Renderer, sidc: SidcInput<'a>) -> Self {
        SymbolBuilder {
            renderer,
            sidc,
            options: SymbolOptions::default(),
            strict: false,
        }
    }

    option_setters!();

    /// Fails on a SIDC that is malformed or that the renderer does not fully
    /// recognise, instead of drawing a `?` icon.
    ///
    /// ```
    /// use milsymbol::{RenderError, Renderer};
    ///
    /// let r = Renderer::default();
    /// assert!(r.symbol("10031000161211000000").strict().render().is_ok());
    /// assert!(matches!(
    ///     r.symbol("10091000001211000000").strict().render(),
    ///     Err(RenderError::MalformedSidc(_))
    /// ));
    /// ```
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Renders the symbol.
    pub fn render(self) -> Result<Symbol, RenderError> {
        if self.strict {
            Sidc::parse(self.sidc.as_str()).map_err(RenderError::MalformedSidc)?;
        }
        let symbol = self.renderer.render(self.sidc.as_str(), self.options)?;
        if self.strict {
            let issues = symbol.sidc_issues();
            if !issues.is_empty() {
                return Err(RenderError::UnsupportedSidc { issues });
            }
        }
        Ok(symbol)
    }
}

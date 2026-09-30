//! Error types.

use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;

/// Error returned by a custom [`SymbolPart`](crate::SymbolPart).
pub type PartError = Box<dyn core::error::Error + Send + Sync>;

/// Why a symbol could not be rendered.
///
/// Malformed SIDCs are not errors (upstream renders them and reports them
/// through validity); errors are option values that cannot be rendered,
/// inputs on which upstream's JavaScript throws, and failing extensions.
#[derive(Debug)]
#[non_exhaustive]
pub enum RenderError {
    /// `colorMode` names a colour mode that is not registered.
    UnknownColorMode {
        /// The requested mode name.
        name: String,
    },
    /// An option value is outside what can be rendered.
    InvalidOption {
        /// Option name, e.g. `stack`.
        name: &'static str,
        /// What is wrong with the value.
        reason: &'static str,
    },
    /// Upstream milsymbol.js throws a JavaScript exception for this input.
    UpstreamException {
        /// The exception upstream raises.
        message: &'static str,
    },
    /// The SIDC is malformed (reported only in strict mode; see
    /// [`SymbolBuilder::strict`](crate::SymbolBuilder::strict)).
    MalformedSidc(crate::sidc::SidcError),
    /// The SIDC is well formed, but this renderer does not recognise all of
    /// it, for example the icon (strict mode only).
    UnsupportedSidc {
        /// Every reason, as [`Symbol::validity`](crate::Symbol::validity) lists them.
        issues: alloc::vec::Vec<crate::ValidityIssue>,
    },
    /// A custom symbol part failed.
    Part {
        /// Position of the part in the renderer's pipeline.
        index: usize,
        /// The part's error.
        source: PartError,
    },
}

impl RenderError {
    pub(crate) fn upstream(message: &'static str) -> Self {
        RenderError::UpstreamException { message }
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::UnknownColorMode { name } => write!(f, "unknown colour mode {name:?}"),
            RenderError::InvalidOption { name, reason } => write!(f, "option `{name}` {reason}"),
            RenderError::UpstreamException { message } => {
                write!(f, "input makes milsymbol.js throw: {message}")
            }
            RenderError::MalformedSidc(e) => write!(f, "malformed SIDC: {e}"),
            RenderError::UnsupportedSidc { issues } => {
                write!(f, "SIDC not supported by this renderer: {issues:?}")
            }
            RenderError::Part { index, source } => {
                write!(f, "symbol part #{index} failed: {source}")
            }
        }
    }
}

impl core::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            RenderError::Part { source, .. } => Some(source.as_ref()),
            RenderError::MalformedSidc(e) => Some(e),
            _ => None,
        }
    }
}

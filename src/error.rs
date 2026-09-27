//! Error types.

use alloc::string::String;
use core::fmt;

/// Why a symbol could not be rendered.
///
/// Malformed SIDCs are not errors (upstream renders them and reports them
/// through validity); errors are option values upstream cannot handle and
/// inputs on which upstream's JavaScript throws.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum RenderError {
    /// `colorMode` names a colour mode that is not registered.
    UnknownColorMode {
        /// The requested mode name.
        name: String,
    },
    /// Upstream milsymbol.js throws a JavaScript exception for this input.
    UpstreamException {
        /// The exception upstream raises.
        message: &'static str,
    },
    /// A custom symbol part failed.
    Part {
        /// Description from the part.
        message: String,
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
            RenderError::UpstreamException { message } => {
                write!(f, "input makes milsymbol.js throw: {message}")
            }
            RenderError::Part { message } => write!(f, "symbol part failed: {message}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RenderError {}

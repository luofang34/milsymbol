//! Native Rust port of [milsymbol.js](https://github.com/spatialillusions/milsymbol)
//! 3.0.4: military symbols per MIL-STD-2525 (B/C/D/E) and APP-6 (B/D/E).
//!
//! ```
//! use milsymbol::{Renderer, options::field};
//!
//! let renderer = Renderer::default();
//! let symbol = renderer
//!     .symbol("10031000001211000000")
//!     .text(field::UNIQUE_DESIGNATION, "1-66")
//!     .render()?;
//! let svg = symbol.to_svg();
//! assert!(svg.starts_with("<svg"));
//! assert!(symbol.is_valid());
//! # Ok::<(), milsymbol::RenderError>(())
//! ```
//!
//! The crate is `no_std + alloc`; the default `std` feature only adds
//! `cache::CachedRenderer`. No JavaScript runs at build or run time: the
//! upstream icon tables were converted to Rust data by `tools/codegen`, and
//! every composition rule is ported to Rust. See the repository `README.md`
//! for the compatibility baseline, extension model and known differences.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod bbox;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod cache;
pub mod catalog;
pub mod color;
pub mod compat;
mod compose;
pub mod config;
pub mod domain;
mod error;
mod generated;
pub mod geometry;
pub mod ir;
mod js;
mod json;
pub mod labels;
mod metadata;
pub mod options;
mod registry;
mod renderer;
pub mod sidc;
mod svg;
mod symbol;
mod template;

pub use bbox::{BBox, PartialBBox};
pub use compose::{BuiltinPart, PartOutput, SymbolPart, SymbolState};
pub use config::{DashArrays, ReferencePlatform, RendererConfig, Standard};
pub use domain::Metadata;
pub use error::{PartError, RenderError};
pub use registry::{IconExtension, IconKey, IconPartContext, PartLookup};
pub use renderer::{Renderer, SymbolBuilder};
pub use svg::SvgOptions;
pub use symbol::{Size, Symbol, Validity, ValidityIssue};

/// Version of milsymbol.js this crate reproduces.
pub const UPSTREAM_VERSION: &str = "3.0.4";

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

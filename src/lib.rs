#![doc = include_str!("lib.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;

mod bbox;
mod builder;
#[cfg(feature = "std")]
#[cfg_attr(docsrs, doc(cfg(feature = "std")))]
pub mod cache;
pub mod catalog;
pub mod color;
pub mod compat;
mod compose;
pub mod config;
pub mod domain;
pub mod drawing;
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
pub use builder::{SidcInput, SymbolBuilder};
pub use compose::{BuiltinPart, PartOutput, SymbolPart, SymbolState};
pub use config::{DashArrays, ReferencePlatform, RendererConfig, Standard};
pub use domain::Metadata;
pub use error::{PartError, RenderError};
pub use registry::{IconExtension, IconKey, IconPartContext, PartLookup};
pub use renderer::{Renderer, RendererBuilder};
pub use svg::SvgOptions;
pub use symbol::{Size, Symbol, Validity, ValidityIssue};

/// Version of milsymbol.js this crate reproduces.
pub const UPSTREAM_VERSION: &str = "3.0.4";

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;

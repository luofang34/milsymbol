//! The book's WASM boundary; parsing and drawing are supplied by milsymbol.

mod api;
mod catalog;
mod protocol;
mod random;
mod snippet;

pub use api::{catalog, random_sidc, render};

//! milsymbol.js-compatible representations, for interoperating with the
//! JavaScript library and for differential testing against it.
//!
//! [`JsMetadata`] is upstream's `symbol.metadata` with its string values and
//! `"undefined"` sentinels; [`canonical_json`] is the record the
//! [differential oracle](https://github.com/luofang34/milsymbol/tree/main/tools/oracle)
//! writes for a symbol. Rendering does not need this module.

use crate::json::Value;
use crate::symbol::Symbol;
use alloc::string::String;

pub use crate::json::Json;
mod metadata;
mod validity;
pub use metadata::{Metadata as JsMetadata, OptionalFlags};
pub use validity::{UpstreamIssue, UpstreamValidity};

/// The symbol's metadata in milsymbol.js's representation, with its string
/// values and `"undefined"` sentinels. [`Symbol::metadata`] has typed values.
pub fn js_metadata(symbol: &Symbol) -> JsMetadata<'_> {
    (&symbol.metadata).into()
}

/// milsymbol.js `isValid()`, with typed reasons.
///
/// Unlike [`Symbol::sidc_validity`], this reproduces upstream's quirks: any
/// text containing `null` (for example the unique designation `"null value"`)
/// or a non-finite number makes the symbol invalid
/// ([`UpstreamIssue::NullInDrawing`]), a hidden icon counts as found, and a
/// malformed SIDC is not reported unless the drawing itself is broken. Use it
/// to compare with milsymbol.js; use [`Symbol::sidc_validity`] to judge input.
///
/// ```
/// use milsymbol::options::TextField;
/// use milsymbol::{Renderer, compat};
///
/// let s = Renderer::default()
///     .symbol("10031000161211000000")
///     .text(TextField::UniqueDesignation, "null value")
///     .render()?;
/// assert!(s.sidc_validity().is_valid());
/// assert!(!compat::is_valid(&s));
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
pub fn validity(symbol: &Symbol) -> UpstreamValidity {
    UpstreamValidity {
        issues: symbol.upstream_issues(),
    }
}

/// Whether [`validity`] has no issues: `validity(symbol).is_valid()`. It is
/// the upstream verdict ([`UpstreamValidity`]), not
/// [`Symbol::sidc_validity`], which judges the SIDC alone.
pub fn is_valid(symbol: &Symbol) -> bool {
    symbol.upstream_issues().is_empty()
}

/// Canonical JSON of the symbol's observable state, in the same shape as
/// the oracle records: instructions, metadata, colours, bounding box, size,
/// anchors, validity and options.
/// Emitted native options take precedence over same-named custom text
/// fields; the text values remain available through [`Symbol::options`].
/// The Rust-only `iconTextUsesFontFamily` option is emitted when enabled;
/// its disabled default is omitted to preserve upstream records.
pub fn canonical_json(symbol: &Symbol) -> Json {
    Value::Symbol(symbol).to_json()
}

/// Appends the canonical record directly to `out`, without constructing an
/// owned JSON tree. Reuse the buffer for bulk output. Object fields are
/// ordered as in JavaScript: array-index keys first in numeric order, then
/// other keys sorted by UTF-16 code units, exactly as in [`canonical_json`].
///
/// ```
/// use milsymbol::{Renderer, compat};
///
/// let symbol = Renderer::default().symbol("10031000001211000000").render()?;
/// let mut record = String::new();
/// compat::write_canonical_json(&symbol, &mut record);
/// assert!(record.contains(r#""affiliation":"Friend""#));
/// # Ok::<(), milsymbol::RenderError>(())
/// ```
pub fn write_canonical_json(symbol: &Symbol, out: &mut String) {
    Value::Symbol(symbol).write(out);
}

/// [`canonical_json`] serialized with sorted keys, as the oracle writes it.
pub fn canonical_json_string(symbol: &Symbol) -> String {
    let mut out = String::with_capacity(4096);
    write_canonical_json(symbol, &mut out);
    out
}

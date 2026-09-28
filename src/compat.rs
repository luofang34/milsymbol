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
pub use metadata::{Metadata as JsMetadata, OptionalFlags};

/// Canonical JSON of the symbol's observable state, in the same shape as
/// the oracle records: instructions, metadata, colours, bounding box, size,
/// anchors, validity and options.
/// Emitted native options take precedence over same-named custom text
/// fields; the text values remain available through [`Symbol::options`].
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

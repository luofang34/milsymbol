//! Differences a case declares with `"known": "<kind>"` (UPSTREAM.md,
//! "Known differences"). A declared case passes only if exactly that
//! difference occurs, so the documentation cannot drift from the behaviour.

use super::{Record, first_diff};
use crate::Error;
use serde_json::Value;

/// A documented difference from milsymbol.js.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Known {
    /// Metadata holds lone UTF-16 surrogates upstream and U+FFFD in Rust.
    LoneSurrogate,
    /// Upstream throws; Rust renders.
    ThrowsUpstream,
    /// Upstream drops an own `__proto__` option; Rust keeps it.
    ProtoKey,
}

impl Known {
    pub(super) fn name(self) -> &'static str {
        match self {
            Known::LoneSurrogate => "lone-surrogate",
            Known::ThrowsUpstream => "throws-upstream",
            Known::ProtoKey => "proto-key",
        }
    }
}

/// The difference a case line declares, if any.
pub(super) fn declared(line: &str) -> Result<Option<Known>, Error> {
    if !line.contains("\"known\"") {
        return Ok(None);
    }
    let case: Value = serde_json::from_str(line)?;
    let Some(kind) = case.get("known") else {
        return Ok(None);
    };
    match kind.as_str() {
        Some("lone-surrogate") => Ok(Some(Known::LoneSurrogate)),
        Some("throws-upstream") => Ok(Some(Known::ThrowsUpstream)),
        Some("proto-key") => Ok(Some(Known::ProtoKey)),
        _ => Err(format!("unknown \"known\" value {kind}").into()),
    }
}

/// `Ok` if the records differ exactly as `kind` describes, otherwise why not.
pub(super) fn check(kind: Known, oracle: &Record<'_>, rust: &Record<'_>) -> Result<(), String> {
    match (kind, oracle, rust) {
        (Known::ThrowsUpstream, Record::Error(_), Record::Rendered { .. }) => Ok(()),
        (
            Known::LoneSurrogate,
            Record::Rendered {
                svg: a,
                sem: b,
                value: c,
                lone_surrogates: true,
            },
            Record::Rendered {
                svg: d,
                sem: e,
                value: f,
                ..
            },
        ) if a == d && b != e => first_diff(c, f, "sem").map_or(Ok(()), Err),
        (
            Known::ProtoKey,
            Record::Rendered {
                svg: a, value: c, ..
            },
            Record::Rendered {
                svg: d, value: f, ..
            },
        ) if a == d => {
            let options = |v: &Value| v.get("options").and_then(|o| o.get("__proto__")).cloned();
            if options(c).is_some() || options(f).is_none() {
                return Err(String::from("expected __proto__ only in the Rust options"));
            }
            let mut without = f.clone();
            if let Some(Value::Object(o)) = without.get_mut("options") {
                o.remove("__proto__");
            }
            first_diff(c, &without, "sem").map_or(Ok(()), Err)
        }
        _ => Err(format!(
            "declared known difference `{}` did not occur as documented",
            kind.name()
        )),
    }
}

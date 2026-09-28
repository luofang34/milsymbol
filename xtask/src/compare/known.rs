//! Only the declared difference may depart from the oracle. Option-key cases
//! carry an expected record from a safe upstream control render, with the
//! input option restored. Its SVG and canonical JSON must match byte for byte.

use super::{Record, first_diff, surrogates};
use crate::Error;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Known {
    LoneSurrogate,
    ThrowsUpstream,
    ProtoKey,
}

impl Known {
    fn option(self) -> Option<&'static str> {
        match self {
            Known::LoneSurrogate => None,
            Known::ThrowsUpstream => Some("hasOwnProperty"),
            Known::ProtoKey => Some("__proto__"),
        }
    }
}

/// The input needed to constrain the permitted difference.
pub(super) struct Declaration {
    kind: Known,
    option: Option<String>,
}

pub(super) fn declared(line: &str) -> Result<Option<Declaration>, Error> {
    let case: Value = serde_json::from_str(line)?;
    let case = case.as_object().ok_or("case must be an object")?;
    let Some(kind) = case.get("known") else {
        return Ok(None);
    };
    let kind = match kind.as_str() {
        Some("lone-surrogate") => Known::LoneSurrogate,
        Some("throws-upstream") => Known::ThrowsUpstream,
        Some("proto-key") => Known::ProtoKey,
        _ => return Err(format!("unknown \"known\" value {kind}").into()),
    };
    let option = kind
        .option()
        .map(|key| {
            case.get("options")
                .and_then(|o| o.get(key))
                .and_then(Value::as_str)
                .map(String::from)
                .ok_or_else(|| format!("known case requires a string {key} option"))
        })
        .transpose()?;
    Ok(Some(Declaration { kind, option }))
}

pub(super) fn check(
    declared: &Declaration,
    oracle: &Record<'_>,
    rust: &Record<'_>,
    expected: Option<&Record<'_>>,
) -> Result<(), String> {
    let Record::Rendered {
        svg,
        sem,
        lone_surrogates: false,
        ..
    } = rust
    else {
        return Err(String::from(
            "Rust must render without lone UTF-16 surrogates",
        ));
    };
    if declared.kind == Known::LoneSurrogate {
        return check_surrogates(oracle, svg, sem);
    }
    let Record::Rendered {
        svg: want_svg,
        sem: want_sem,
        value,
        lone_surrogates: false,
    } = expected.ok_or("known option-key difference requires an oracle control record")?
    else {
        return Err(String::from(
            "oracle control must render without lone UTF-16 surrogates",
        ));
    };
    let key = declared.kind.option().ok_or("missing known option key")?;
    if value
        .get("options")
        .and_then(|o| o.get(key))
        .and_then(Value::as_str)
        != declared.option.as_deref()
    {
        return Err(format!(
            "oracle control must preserve the input {key} value"
        ));
    }
    match (declared.kind, oracle) {
        (Known::ThrowsUpstream, Record::Error("options.hasOwnProperty is not a function")) => {}
        (
            Known::ProtoKey,
            Record::Rendered {
                svg: actual_svg,
                value: actual,
                lone_surrogates: false,
                ..
            },
        ) if actual_svg == want_svg => check_proto(actual, value)?,
        _ => return Err(String::from("declared upstream behaviour did not occur")),
    }
    if svg != want_svg || sem != want_sem {
        return Err(String::from(
            "Rust output differs from the exact oracle control record",
        ));
    }
    Ok(())
}

fn check_surrogates(oracle: &Record<'_>, svg: &str, sem: &str) -> Result<(), String> {
    match oracle {
        Record::Rendered {
            svg: actual_svg,
            sem: actual_sem,
            lone_surrogates: true,
            ..
        } if *actual_svg == svg && surrogates::replace_lone(actual_sem).as_deref() == Some(sem) => {
            Ok(())
        }
        _ => Err(String::from(
            "expected only lone-surrogate replacement; SVG and other JSON bytes must match",
        )),
    }
}

fn check_proto(actual: &Value, expected: &Value) -> Result<(), String> {
    if actual
        .get("options")
        .and_then(|o| o.get("__proto__"))
        .is_some()
    {
        return Err(String::from("expected __proto__ to be absent upstream"));
    }
    let mut without = expected.clone();
    if let Some(Value::Object(options)) = without.get_mut("options") {
        options.shift_remove("__proto__");
    }
    first_diff(actual, &without, "sem").map_or(Ok(()), Err)
}

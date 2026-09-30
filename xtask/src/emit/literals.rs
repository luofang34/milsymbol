//! Rust literal formatting for emitted tables.

use crate::Error;
use crate::jsnum;
use serde_json::Value;

/// A Rust string literal.
pub(super) fn rstr(s: &str) -> String {
    let mut out = String::from("\"");
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{{{:x}}}", c as u32))
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A Rust `f64` literal with JavaScript's shortest digits.
pub(super) fn rf64(v: &Value) -> Result<String, Error> {
    let n = v.as_f64().ok_or_else(|| format!("not a number: {v}"))?;
    if !n.is_finite() {
        return Err(format!("non-finite number {n}").into());
    }
    if n == 0.0 && n.is_sign_negative() {
        return Ok(String::from("-0.0"));
    }
    let mut s = jsnum::to_string(n);
    if !s.contains(['.', 'e']) {
        s.push_str(".0");
    }
    let mantissa_is_integer = s.split_once('e').is_some_and(|(m, _)| !m.contains('.'));
    if mantissa_is_integer {
        s = s.replacen('e', ".0e", 1);
    }
    Ok(s)
}

/// `Cow::Borrowed("…")`.
pub(super) fn rcow(s: &str) -> String {
    format!("Cow::Borrowed({})", rstr(s))
}

/// `None`, or `Some(f(v))` for a present key.
pub(super) fn ropt(
    v: Option<&Value>,
    f: impl Fn(&Value) -> Result<String, Error>,
) -> Result<String, Error> {
    Ok(match v {
        None => String::from("None"),
        Some(v) => format!("Some({})", f(v)?),
    })
}

/// The string field `s` of a template value.
pub(super) fn field_s(v: &Value) -> Result<&str, Error> {
    v.get("s")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("expected {{\"s\": …}}, got {v}").into())
}

/// `field_s` as a string literal.
pub(super) fn rs(v: &Value) -> Result<String, Error> {
    Ok(rstr(field_s(v)?))
}

/// A boolean literal.
pub(super) fn rbool(v: &Value) -> Result<String, Error> {
    v.as_bool()
        .map(|b| b.to_string())
        .ok_or_else(|| format!("not a boolean: {v}").into())
}

/// A `TNum` literal.
pub(super) fn rnum(t: &Value) -> Result<String, Error> {
    if let Some(n) = t.get("n") {
        return Ok(format!("TNum::N({})", rf64(n)?));
    }
    if let Some(s) = t.get("s").and_then(Value::as_str) {
        return Ok(format!("TNum::S({})", rstr(s)));
    }
    if let Some(b) = t.get("b").and_then(Value::as_bool) {
        return Ok(format!("TNum::B({b})"));
    }
    Err(format!("bad num {t}").into())
}

fn slot_name(slot: &str) -> Option<&'static str> {
    Some(match slot {
        "fillColor" => "Fill",
        "frameColor" => "Frame",
        "iconColor" => "Icon",
        "iconFillColor" => "IconFill",
        "black" => "Black",
        "white" => "White",
        "none" => "None",
        _ => return None,
    })
}

fn aff_name(aff: &str) -> Option<&'static str> {
    Some(match aff {
        "self" => "SelfAff",
        "Civilian" => "Civilian",
        "Friend" => "Friend",
        "Hostile" => "Hostile",
        "Neutral" => "Neutral",
        "Unknown" => "Unknown",
        "Suspect" => "Suspect",
        _ => return None,
    })
}

/// A `TPaint` literal.
pub(super) fn rpaint(p: &Value) -> Result<String, Error> {
    let get = |k: &str| p.get(k).and_then(Value::as_str);
    match get("k") {
        Some("none") => Ok(String::from("TPaint::None")),
        Some("mono") => Ok(String::from("TPaint::Mono")),
        Some("lit") => Ok(format!(
            "TPaint::Lit({})",
            rstr(get("v").unwrap_or_default())
        )),
        Some("slot") => {
            let slot = get("slot").and_then(slot_name);
            let aff = get("aff").and_then(aff_name);
            match (slot, aff) {
                (Some(s), Some(a)) => Ok(format!("TPaint::Slot(Slot::{s}, TAff::{a})")),
                _ => Err(format!("bad slot {p}").into()),
            }
        }
        _ => Err(format!("bad paint {p}").into()),
    }
}

/// A `TDash` literal.
pub(super) fn rdash(t: &Value) -> Result<String, Error> {
    match t.get("dash").and_then(Value::as_str) {
        Some("pending") => Ok(String::from("TDash::Pending")),
        Some("anticipated") => Ok(String::from("TDash::Anticipated")),
        _ => Ok(format!("TDash::Lit({})", rs(t)?)),
    }
}

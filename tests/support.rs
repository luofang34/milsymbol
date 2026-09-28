//! Oracle case decoding shared by the differential tests and the `dump` example.
#![allow(dead_code)]

use milsymbol::color::ColorMode;
use milsymbol::ir::Paint;
use milsymbol::options::{OptionError, OptionValue, SymbolOptions};
use milsymbol::{DashArrays, ReferencePlatform, Renderer, RendererConfig, Standard};
use serde_json::Value;

/// FNV-1a 64-bit over UTF-8 bytes (mirrors `tools/oracle/oracle.mjs`).
pub fn fnv64(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

fn color_mode(obj: &serde_json::Map<String, Value>) -> ColorMode {
    let mut m = ColorMode::default();
    for (k, v) in obj {
        if let Some(slot) = m.slot_mut(k) {
            *slot = match v {
                Value::String(s) => Some(Paint::color(s.clone())),
                Value::Bool(false) => Some(Paint::None),
                _ => None,
            };
        }
    }
    m
}

/// Options of a case, or the error message expected from `set`.
pub fn options(case: &Value) -> Result<SymbolOptions, String> {
    let mut o = SymbolOptions::default();
    if let Some(Value::Object(map)) = case.get("options") {
        for (k, v) in map {
            let value = match v {
                Value::String(s) => OptionValue::Str(s.clone()),
                Value::Number(n) => OptionValue::Num(n.as_f64().unwrap_or(f64::NAN)),
                Value::Bool(b) => OptionValue::Bool(*b),
                Value::Object(obj) => OptionValue::Colors(color_mode(obj)),
                other => return Err(format!("unsupported option value {other}")),
            };
            match o.set(k, value.clone()) {
                // Custom text fields such as `dtg1` (used by label overrides).
                Err(OptionError::Unknown { .. }) => match value {
                    OptionValue::Str(s) => {
                        o.set_text(k, s);
                    }
                    _ => return Err(format!("unknown option {k}")),
                },
                other => {
                    other.map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(o)
}

/// Renderer configured for a case, reproducing V8 on `platform`.
pub fn renderer(case: &Value, platform: ReferencePlatform) -> Renderer {
    let mut config = RendererConfig::default();
    config.reference_platform = platform;
    if let Some(cfg) = case.get("cfg") {
        if let Some(s) = cfg.get("standard").and_then(Value::as_str) {
            if s == "APP6" {
                config.standard = Standard::App6;
            }
        }
        if let Some(Value::Array(d)) = cfg.get("dashArrays") {
            let g = |i: usize| {
                d.get(i)
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string()
            };
            config.dash_arrays = DashArrays::new(g(0), g(1), g(2));
        }
        if let Some(h) = cfg.get("hqStaffLength").and_then(Value::as_f64) {
            config.hq_staff_length = h;
        }
    }
    Renderer::new(config)
}

/// Rendered record of a case for V8 on x64 (the fixtures' platform).
pub fn render(case: &Value) -> Result<(String, String), String> {
    render_for(case, ReferencePlatform::X64)
}

/// Rendered record of a case: (svg, canonical json) or an error string.
pub fn render_for(case: &Value, platform: ReferencePlatform) -> Result<(String, String), String> {
    let sidc = case.get("sidc").and_then(Value::as_str).unwrap_or_default();
    let options = options(case)?;
    let r = renderer(case, platform);
    let symbol = r.render(sidc, options).map_err(|e| e.to_string())?;
    Ok((
        symbol.to_svg(),
        milsymbol::compat::canonical_json_string(&symbol),
    ))
}
